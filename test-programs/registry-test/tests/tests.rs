#![cfg(feature = "test-sbf")]

use anchor_lang::{InstructionData, ToAccountMetas};
use light_registry::{
    get_forester_epoch_pda_address,
    sdk::{
        create_nullify_instruction, create_update_address_merkle_tree_instruction,
        get_governance_authority_pda, CreateNullifyInstructionInputs,
        UpdateAddressMerkleTreeInstructionInputs,
    },
    ForesterEpoch, LightGovernanceAuthority, RegistryError,
};
use light_test_utils::{
    registry::{
        create_rollover_address_merkle_tree_instructions,
        create_rollover_state_merkle_tree_instructions, register_test_forester,
        update_test_forester,
    },
    rpc::{errors::assert_rpc_error, rpc_connection::RpcConnection, SolanaRpcConnection},
    test_env::{
        get_test_env_accounts, register_program_with_registry_program,
        setup_test_programs_with_accounts,
    },
};
use light_test_utils::{rpc::solana_rpc::SolanaRpcUrl, test_env::setup_accounts_devnet};
use solana_sdk::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::str::FromStr;

#[tokio::test]
async fn test_register_program() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let random_program_keypair = Keypair::new();
    register_program_with_registry_program(
        &mut rpc,
        &env.governance_authority,
        &env.group_pda,
        &random_program_keypair,
    )
    .await
    .unwrap();
}

/// Test:
/// 1. SUCCESS: Register a forester
/// 2. SUCCESS: Update forester authority
#[tokio::test]
async fn test_register_and_update_forester_pda() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    // 1. SUCCESS: Register a forester
    register_test_forester(
        &mut rpc,
        &env.governance_authority,
        &forester_keypair.pubkey(),
    )
    .await
    .unwrap();

    // 2. SUCCESS: Update forester authority
    let new_forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&new_forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    update_test_forester(&mut rpc, &forester_keypair, &new_forester_keypair.pubkey())
        .await
        .unwrap();
}

/// Test:
/// 1. FAIL: Register a forester with invalid authority
/// 2. FAIL: Update forester authority with invalid authority
/// 3. FAIL: Nullify with invalid authority
/// 4. FAIL: Update address tree with invalid authority
/// 5. FAIL: Rollover address tree with invalid authority
/// 6. FAIL: Rollover state tree with invalid authority
#[tokio::test]
async fn failing_test_forester() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    // 1. FAIL: Register a forester with invalid authority
    {
        let result = register_test_forester(&mut rpc, &payer, &Keypair::new().pubkey()).await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 2. FAIL: Update forester authority with invalid authority
    {
        let forester_epoch_pda = get_forester_epoch_pda_address(&env.forester.pubkey()).0;
        let instruction_data = light_registry::instruction::UpdateForesterEpochPda {
            authority: Keypair::new().pubkey(),
        };
        let accounts = light_registry::accounts::UpdateForesterEpochPda {
            forester_epoch_pda,
            signer: payer.pubkey(),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 3. FAIL: Nullify with invalid authority
    {
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: env.nullifier_queue_pubkey,
            merkle_tree: env.merkle_tree_pubkey,
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: payer.pubkey(),
        };
        let mut ix = create_nullify_instruction(inputs);
        // Swap the derived forester pda with an initialized but invalid one.
        ix.accounts[0].pubkey = get_forester_epoch_pda_address(&env.forester.pubkey()).0;
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: update address Merkle tree failed
    {
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority: authority.pubkey(),
                address_merkle_tree: env.address_merkle_tree_pubkey,
                address_queue: env.address_merkle_tree_queue_pubkey,
                changelog_index: 0,
                indexed_changelog_index: 0,
                value: 1,
                low_address_index: 1,
                low_address_value: [0u8; 32],
                low_address_next_index: 1,
                low_address_next_value: [0u8; 32],
                low_address_proof: [[0u8; 32]; 16],
            },
        );
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey = get_forester_epoch_pda_address(&env.forester.pubkey()).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 5. FAIL: rollover address tree with invalid authority
    {
        let new_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_address_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &new_queue_keypair,
            &new_merkle_tree_keypair,
            &env.address_merkle_tree_pubkey,
            &env.address_merkle_tree_queue_pubkey,
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_address(&env.forester.pubkey()).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[&authority, &new_queue_keypair, &new_merkle_tree_keypair],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
    // 6. FAIL: rollover state tree with invalid authority
    {
        let new_nullifier_queue_keypair = Keypair::new();
        let new_state_merkle_tree_keypair = Keypair::new();
        let new_cpi_context = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_state_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
            &env.merkle_tree_pubkey,
            &env.nullifier_queue_pubkey,
            &new_cpi_context.pubkey(),
        )
        .await;
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey =
            get_forester_epoch_pda_address(&env.forester.pubkey()).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[
                    &authority,
                    &new_nullifier_queue_keypair,
                    &new_state_merkle_tree_keypair,
                ],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
}

// cargo test-sbf -p registry-test -- --test update_registry_governance_on_testnet update_forester_on_testnet --ignored --nocapture
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_forester_on_testnet() {
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::ZKTestnet, None);
    rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 100)
        .await
        .unwrap();
    let forester_epoch_account =
        Pubkey::from_str("DFiGEbaz75wSdqy9bpeWacqLWrqAwWBfqh4iSYtejiwK").unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterEpoch>(&env_accounts.registered_forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch);
    assert_eq!(forester_epoch.authority, env_accounts.forester.pubkey());

    let updated_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    update_test_forester(&mut rpc, &env_accounts.forester, &updated_keypair.pubkey())
        .await
        .unwrap();
    let forester_epoch = rpc
        .get_anchor_account::<ForesterEpoch>(&env_accounts.registered_forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    println!("ForesterEpoch: {:?}", forester_epoch_account);
    assert_eq!(forester_epoch.authority, updated_keypair.pubkey());
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_registry_governance_on_testnet() {
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::ZKTestnet, None);
    rpc.airdrop_lamports(
        &env_accounts.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await
    .unwrap();
    let governance_authority = rpc
        .get_anchor_account::<LightGovernanceAuthority>(&env_accounts.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    println!("GroupAuthority: {:?}", governance_authority);
    assert_eq!(
        governance_authority.authority,
        env_accounts.governance_authority.pubkey()
    );

    let updated_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    let (_, bump) = get_governance_authority_pda();
    let instruction = light_registry::instruction::UpdateGovernanceAuthority {
        new_authority: updated_keypair.pubkey(),
        bump,
    };
    let accounts = light_registry::accounts::UpdateAuthority {
        authority_pda: env_accounts.governance_authority_pda,
        authority: env_accounts.governance_authority.pubkey(),
    };
    let ix = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    let signature = rpc
        .create_and_send_transaction(
            &[ix],
            &env_accounts.governance_authority.pubkey(),
            &[&env_accounts.governance_authority],
        )
        .await
        .unwrap();
    println!("signature: {:?}", signature);
    let governance_authority = rpc
        .get_anchor_account::<LightGovernanceAuthority>(&env_accounts.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(governance_authority.authority, updated_keypair.pubkey());
}

// cargo test-sbf -p registry-test -- --test init_accounts --ignored --nocapture
// TODO: refactor into xtask
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn init_accounts() {
    let authority_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    let forester_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("authority pubkey: {:?}", authority_keypair.pubkey());
    println!("forester pubkey: {:?}", forester_keypair.pubkey());
    setup_accounts_devnet(&authority_keypair, &forester_keypair).await;
}
