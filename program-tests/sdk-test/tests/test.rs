#![cfg(feature = "test-sbf")]
use std::{println, vec};

use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::RpcConnection,
};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::setup_test_programs_with_accounts_v2,
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use light_sdk::{
    account_meta::LightAccountMeta,
    address::derive_address,
    instruction_data::LightInstructionData,
    merkle_context::{AddressMerkleContext, RemainingAccounts},
    system_accounts::SystemAccountMetaConfig,
};
use solana_sdk::{instruction::Instruction, signature::Signer};

#[tokio::test]
async fn test_sdk_test() {
    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![(String::from("sdk_test"), sdk_test::ID)]))
            .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::new(
        vec![StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }],
        vec![AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }],
        payer.insecure_clone(),
        env.group_pda,
        Some(ProverConfig {
            circuits: vec![ProofType::Inclusion, ProofType::NonInclusion],
            run_mode: None,
        }),
    )
    .await;
    let system_account_meta_config = SystemAccountMetaConfig {
        self_program: sdk_test::ID,
        ..SystemAccountMetaConfig::default()
    };
    let mut accounts = RemainingAccounts::default();
    accounts.insert_or_get_signer_mut(payer.pubkey());
    accounts.add_system_accounts(system_account_meta_config);

    let mut remaining_accounts = RemainingAccounts::default();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let account_data = [1u8; 31];
    println!(
        "offchain address_merkle_context {:?}",
        address_merkle_context
    );

    let (address, _) = derive_address(
        &[b"compressed", &account_data],
        &address_merkle_context,
        &sdk_test::ID,
    );
    println!("offchain address {:?}", address);
    {
        let rpc_result = test_indexer
            .create_proof_for_compressed_accounts(
                None,
                None,
                Some(&[address]),
                Some(vec![env.address_merkle_tree_pubkey]),
                &mut rpc,
            )
            .await
            .unwrap();

        let address_merkle_context = AddressMerkleContext {
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
        };
        let account = LightAccountMeta::new_init(
            &env.merkle_tree_pubkey,
            Some(&address_merkle_context),
            Some(rpc_result.address_root_indices[0]),
            &mut remaining_accounts,
        )
        .unwrap();

        let inputs = LightInstructionData {
            proof: Some(rpc_result),
            accounts: Some(vec![account]),
        };
        let inputs = inputs.serialize().unwrap();

        let system_accounts = accounts.to_account_metas();
        let remaining_accounts = remaining_accounts.to_account_metas();
        let accounts = vec![system_accounts, remaining_accounts].concat();
        let instruction = Instruction {
            program_id: sdk_test::ID,
            accounts,
            data: [&[0u8][..], &inputs[..], &account_data[..]].concat(),
        };

        let (event, _, slot) = rpc
            .create_and_send_transaction_with_public_event(
                &[instruction],
                &payer.pubkey(),
                &[&payer],
                None,
            )
            .await
            .unwrap()
            .unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &event);
    }
}
