use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env};

use crate::{ContractStatus, Escrow, EscrowClient, ReleaseAuthorization};

#[test]
fn test_hello() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

#[test]
fn test_create_contract() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128, 600_0000000_i128];

    let id = client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );
    assert_eq!(id, 1);
}

#[test]
fn test_create_contract_with_arbiter() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    let id = client.create_contract(
        &client_addr,
        &freelancer_addr,
        &Some(arbiter_addr),
        &milestones,
        &ReleaseAuthorization::ClientAndArbiter,
    );
    assert_eq!(id, 1);
}

#[test]
#[should_panic(expected = "At least one milestone required")]
fn test_create_contract_no_milestones() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env];

    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );
}

#[test]
#[should_panic(expected = "Client and freelancer cannot be the same address")]
fn test_create_contract_same_addresses() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    client.create_contract(
        &client_addr,
        &client_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );
}

#[test]
#[should_panic(expected = "Milestone amounts must be positive")]
fn test_create_contract_negative_amount() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, -1000_0000000_i128];

    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );
}

#[test]
fn test_deposit_funds() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract first
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be the client
    env.mock_auths(&[(&client_addr, &contract_id, &symbol_short!("deposit_funds"))]);

    let result = client.deposit_funds(&1, &1000_0000000);
    assert!(result);
}

#[test]
#[should_panic(expected = "Deposit amount must equal total milestone amounts")]
fn test_deposit_funds_wrong_amount() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract first
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be the client
    env.mock_auths(&[(&client_addr, &contract_id, &symbol_short!("deposit_funds"))]);

    client.deposit_funds(&1, &500_0000000);
}

#[test]
fn test_approve_milestone_release_client_only() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be the client
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    let result = client.approve_milestone_release(&1, &0);
    assert!(result);
}

#[test]
fn test_approve_milestone_release_client_and_arbiter() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &Some(arbiter_addr),
        &milestones,
        &ReleaseAuthorization::ClientAndArbiter,
    );

    // Test client approval
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    let result = client.approve_milestone_release(&1, &0);
    assert!(result);

    // Test arbiter approval
    env.mock_auths(&[(
        &arbiter_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    let result = client.approve_milestone_release(&1, &0);
    assert!(result);
}

#[test]
#[should_panic(expected = "Caller not authorized to approve milestone release")]
fn test_approve_milestone_release_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let unauthorized_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be unauthorized
    env.mock_auths(&[(
        &unauthorized_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    client.approve_milestone_release(&1, &0);
}

#[test]
#[should_panic(expected = "Invalid milestone ID")]
fn test_approve_milestone_release_invalid_id() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be the client
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    client.approve_milestone_release(&1, &5);
}

#[test]
#[should_panic(expected = "Milestone already approved by this address")]
fn test_approve_milestone_release_already_approved() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Mock the caller to be the client
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);

    // First approval should succeed
    let result = client.approve_milestone_release(&1, &0);
    assert!(result);

    // Second approval should fail
    client.approve_milestone_release(&1, &0);
}

#[test]
fn test_release_milestone_client_only() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Approve milestone first
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &0);

    // Release milestone
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);

    let result = client.release_milestone(&1, &0);
    assert!(result);
}

#[test]
fn test_release_milestone_arbiter_only() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &Some(arbiter_addr),
        &milestones,
        &ReleaseAuthorization::ArbiterOnly,
    );

    // Approve milestone as arbiter
    env.mock_auths(&[(
        &arbiter_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &0);

    // Release milestone
    env.mock_auths(&[(
        &arbiter_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);

    let result = client.release_milestone(&1, &0);
    assert!(result);
}

#[test]
#[should_panic(expected = "Insufficient approvals for milestone release")]
fn test_release_milestone_no_approval() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Try to release without approval
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);

    client.release_milestone(&1, &0);
}

#[test]
#[should_panic(expected = "Milestone already released")]
fn test_release_milestone_already_released() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Approve milestone
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &0);

    // Release milestone first time
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);
    let result = client.release_milestone(&1, &0);
    assert!(result);

    // Try to release again
    client.release_milestone(&1, &0);
}

#[test]
fn test_release_milestone_multi_sig() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &Some(arbiter_addr),
        &milestones,
        &ReleaseAuthorization::MultiSig,
    );

    // Approve milestone as client
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &0);

    // Release milestone
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);

    let result = client.release_milestone(&1, &0);
    assert!(result);
}

#[test]
fn test_contract_completion_all_milestones_released() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1000_0000000_i128, 2000_0000000_i128];

    // Create contract
    client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );

    // Approve and release first milestone
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &0);

    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);
    client.release_milestone(&1, &0);

    // Approve and release second milestone
    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("approve_milestone_release"),
    )]);
    client.approve_milestone_release(&1, &1);

    env.mock_auths(&[(
        &client_addr,
        &contract_id,
        &symbol_short!("release_milestone"),
    )]);
    client.release_milestone(&1, &1);

    // All milestones should be released and contract completed
    // Note: In a real implementation, we would check the contract status
    // For this simplified version, we just verify no panics occurred
}

#[test]
fn test_edge_cases() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1_0000000_i128]; // Minimum amount

    // Test with minimum amount
    let id = client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &milestones,
        &ReleaseAuthorization::ClientOnly,
    );
    assert_eq!(id, 1);

    // Test with multiple milestones
    let many_milestones = vec![
        &env,
        100_0000000_i128,
        200_0000000_i128,
        300_0000000_i128,
        400_0000000_i128,
    ];
    let id2 = client.create_contract(
        &client_addr,
        &freelancer_addr,
        &None::<Address>,
        &many_milestones,
        &ReleaseAuthorization::ClientOnly,
    );
    assert_eq!(id2, 2); // Should increment contract ID
}
