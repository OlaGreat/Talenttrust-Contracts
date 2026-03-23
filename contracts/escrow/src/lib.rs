#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

/// Persistent lifecycle state for an escrow agreement.
///
/// Security notes:
/// - Only `Created -> Funded -> Completed` transitions are currently supported.
/// - `Disputed` is reserved for future dispute resolution flows and is not reachable
///   in the current implementation.

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
}

/// Individual milestone tracked inside an escrow agreement.
///
/// Invariant:
/// - `released == true` is irreversible.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

/// Stored escrow state for a single agreement.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowContractData {
    pub client: Address,
    pub freelancer: Address,
    pub milestones: Vec<Milestone>,
    pub total_amount: i128,
    pub funded_amount: i128,
    pub released_amount: i128,
    pub status: ContractStatus,
}

/// Reputation state derived from completed escrow contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationRecord {
    pub completed_contracts: u32,
    pub total_rating: i128,
    pub last_rating: i128,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    NextContractId,
    Contract(u32),
    Reputation(Address),
    PendingReputationCredits(Address),
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Creates a new escrow contract and stores milestone funding requirements.
    ///
    /// Security properties:
    /// - The declared client must authorize creation.
    /// - Client and freelancer addresses must be distinct.
    /// - All milestones must have a strictly positive amount.
    /// - Funding amount is fixed at creation time by the milestone sum.
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
    ) -> u32 {
        client.require_auth();

        if client == freelancer {
            panic!("client and freelancer must differ");
        }
        if milestone_amounts.is_empty() {
            panic!("at least one milestone is required");
        }

        let mut milestones = Vec::new(&env);
        let mut total_amount = 0_i128;
        let mut index = 0_u32;
        while index < milestone_amounts.len() {
            let amount = milestone_amounts
                .get(index)
                .unwrap_or_else(|| panic!("missing milestone amount"));
            if amount <= 0 {
                panic!("milestone amount must be positive");
            }
            total_amount = total_amount
                .checked_add(amount)
                .unwrap_or_else(|| panic!("milestone total overflow"));
            milestones.push_back(Milestone {
                amount,
                released: false,
            });
            index += 1;
        }

        let contract_id = Self::next_contract_id(&env);
        let contract = EscrowContractData {
            client,
            freelancer,
            milestones,
            total_amount,
            funded_amount: 0,
            released_amount: 0,
            status: ContractStatus::Created,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &contract);
        env.storage()
            .persistent()
            .set(&DataKey::NextContractId, &(contract_id + 1));

        contract_id
    }

    /// Deposits the full escrow amount for a contract.
    ///
    /// Security properties:
    /// - Only the recorded client may fund the contract.
    /// - Funding is allowed exactly once.
    /// - Partial or excess funding is rejected to avoid ambiguous release logic.
    pub fn deposit_funds(env: Env, contract_id: u32, amount: i128) -> bool {
        if amount <= 0 {
            panic!("deposit amount must be positive");
        }

        let mut contract = Self::load_contract(&env, contract_id);
        contract.client.require_auth();

        if contract.status != ContractStatus::Created {
            panic!("contract is not awaiting funding");
        }
        if amount != contract.total_amount {
            panic!("deposit must match milestone total");
        }

        contract.funded_amount = amount;
        contract.status = ContractStatus::Funded;
        Self::save_contract(&env, contract_id, &contract);

        true
    }

    /// Releases a single milestone payment.
    ///
    /// Security properties:
    /// - Only the client may authorize a release.
    /// - Milestones can be released once.
    /// - Contract completion is derived from all milestones being released.
    pub fn release_milestone(env: Env, contract_id: u32, milestone_id: u32) -> bool {
        let mut contract = Self::load_contract(&env, contract_id);
        contract.client.require_auth();

        if contract.status != ContractStatus::Funded {
            panic!("contract is not funded");
        }
        if milestone_id >= contract.milestones.len() {
            panic!("milestone id out of range");
        }

        let mut milestone = contract
            .milestones
            .get(milestone_id)
            .unwrap_or_else(|| panic!("missing milestone"));
        if milestone.released {
            panic!("milestone already released");
        }

        let next_released_amount = contract
            .released_amount
            .checked_add(milestone.amount)
            .unwrap_or_else(|| panic!("released total overflow"));
        if next_released_amount > contract.funded_amount {
            panic!("release exceeds funded amount");
        }

        milestone.released = true;
        contract.milestones.set(milestone_id, milestone);
        contract.released_amount = next_released_amount;

        if Self::all_milestones_released(&contract.milestones) {
            contract.status = ContractStatus::Completed;
            Self::add_pending_reputation_credit(&env, &contract.freelancer);
        }

        Self::save_contract(&env, contract_id, &contract);

        true
    }

    /// Issues a bounded reputation rating for a freelancer after a completed contract.
    ///
    /// Security properties:
    /// - The freelancer must authorize the write to their own reputation record.
    /// - A reputation update is only possible after a completed contract grants a
    ///   pending reputation credit.
    /// - Ratings are limited to the inclusive range `1..=5`.
    ///
    /// Residual risk:
    /// - The current interface lets the freelancer self-submit the rating value.
    ///   The contract therefore treats this record as informational only and does
    ///   not use it for fund movement or access control.
    pub fn issue_reputation(env: Env, freelancer: Address, rating: i128) -> bool {
        freelancer.require_auth();

        if !(1..=5).contains(&rating) {
            panic!("rating must be between 1 and 5");
        }

        let pending_key = DataKey::PendingReputationCredits(freelancer.clone());
        let pending_credits = env
            .storage()
            .persistent()
            .get::<_, u32>(&pending_key)
            .unwrap_or(0);
        if pending_credits == 0 {
            panic!("no completed contract available for reputation");
        }

        let rep_key = DataKey::Reputation(freelancer.clone());
        let mut record = env
            .storage()
            .persistent()
            .get::<_, ReputationRecord>(&rep_key)
            .unwrap_or(ReputationRecord {
                completed_contracts: 0,
                total_rating: 0,
                last_rating: 0,
            });

        record.completed_contracts += 1;
        record.total_rating = record
            .total_rating
            .checked_add(rating)
            .unwrap_or_else(|| panic!("rating total overflow"));
        record.last_rating = rating;

        env.storage().persistent().set(&rep_key, &record);
        env.storage()
            .persistent()
            .set(&pending_key, &(pending_credits - 1));

        true
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }

    /// Returns the stored contract state.
    pub fn get_contract(env: Env, contract_id: u32) -> EscrowContractData {
        Self::load_contract(&env, contract_id)
    }

    /// Returns the stored reputation record for a freelancer, if present.
    pub fn get_reputation(env: Env, freelancer: Address) -> Option<ReputationRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Reputation(freelancer))
    }

    /// Returns the number of pending reputation updates that can be claimed.
    pub fn get_pending_reputation_credits(env: Env, freelancer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::PendingReputationCredits(freelancer))
            .unwrap_or(0)
    }
}

impl Escrow {
    fn next_contract_id(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::NextContractId)
            .unwrap_or(1)
    }

    fn load_contract(env: &Env, contract_id: u32) -> EscrowContractData {
        env.storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .unwrap_or_else(|| panic!("contract not found"))
    }

    fn save_contract(env: &Env, contract_id: u32, contract: &EscrowContractData) {
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), contract);
    }

    fn add_pending_reputation_credit(env: &Env, freelancer: &Address) {
        let key = DataKey::PendingReputationCredits(freelancer.clone());
        let current = env.storage().persistent().get::<_, u32>(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + 1));
    }

    fn all_milestones_released(milestones: &Vec<Milestone>) -> bool {
        let mut index = 0_u32;
        while index < milestones.len() {
            let milestone = milestones
                .get(index)
                .unwrap_or_else(|| panic!("missing milestone"));
            if !milestone.released {
                return false;
            }
            index += 1;
        }
        true
    }
}

#[cfg(test)]
mod test;
