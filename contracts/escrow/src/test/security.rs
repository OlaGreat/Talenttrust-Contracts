extern crate std;

use std::panic::{catch_unwind, AssertUnwindSafe};

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

fn setup(mock_auths: bool) -> (Env, Address) {
    let env = Env::default();
    if mock_auths {
        env.mock_all_auths();
    }
    let contract_id = env.register(Escrow, ());
    (env, contract_id)
}

fn assert_panics<F>(f: F)
where
    F: FnOnce(),
{
    assert!(catch_unwind(AssertUnwindSafe(f)).is_err());
}

#[test]
fn create_contract_requires_client_auth() {
    let (env, contract_id) = setup(false);
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_i128];

    assert_panics(|| {
        client.create_contract(&client_addr, &freelancer_addr, &milestones);
    });
}

#[test]
fn create_contract_rejects_invalid_party_or_milestone_input() {
    let (env, contract_id) = setup(true);
    let client = EscrowClient::new(&env, &contract_id);

    let same_party = Address::generate(&env);
    let empty_milestones = vec![&env];
    let invalid_milestones = vec![&env, 100_i128, 0_i128];

    assert_panics(|| {
        client.create_contract(&same_party, &same_party, &vec![&env, 100_i128]);
    });
    assert_panics(|| {
        client.create_contract(&same_party, &Address::generate(&env), &empty_milestones);
    });
    assert_panics(|| {
        client.create_contract(&same_party, &Address::generate(&env), &invalid_milestones);
    });
}

#[test]
fn deposit_rejects_partial_duplicate_or_unknown_contract_funding() {
    let (env, contract_id) = setup(true);
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_i128, 100_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    assert_panics(|| {
        client.deposit_funds(&id, &150_i128);
    });

    assert!(client.deposit_funds(&id, &200_i128));

    assert_panics(|| {
        client.deposit_funds(&id, &200_i128);
    });
    assert_panics(|| {
        client.deposit_funds(&999_u32, &200_i128);
    });
}

#[test]
fn release_rejects_unfunded_duplicate_and_out_of_range_access() {
    let (env, contract_id) = setup(true);
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    assert_panics(|| {
        client.release_milestone(&id, &0_u32);
    });

    client.deposit_funds(&id, &100_i128);
    assert!(client.release_milestone(&id, &0_u32));

    assert_panics(|| {
        client.release_milestone(&id, &0_u32);
    });
    assert_panics(|| {
        client.release_milestone(&id, &5_u32);
    });
}

#[test]
fn reputation_requires_completed_contract_credit_and_valid_rating() {
    let (env, contract_id) = setup(true);
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    assert_panics(|| {
        client.issue_reputation(&freelancer_addr, &5_i128);
    });

    client.deposit_funds(&id, &100_i128);
    client.release_milestone(&id, &0_u32);

    assert_panics(|| {
        client.issue_reputation(&freelancer_addr, &0_i128);
    });
    assert_panics(|| {
        client.issue_reputation(&freelancer_addr, &6_i128);
    });

    assert!(client.issue_reputation(&freelancer_addr, &4_i128));

    assert_panics(|| {
        client.issue_reputation(&freelancer_addr, &4_i128);
    });
}
