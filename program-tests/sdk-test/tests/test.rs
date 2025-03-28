#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_client::{
    indexer::Indexer,
    rpc::{RpcConnection, RpcError},
};
use light_compressed_account::{
    address::derive_address, compressed_account::CompressedAccountWithMerkleContext,
    hashv_to_bn254_field_size_be,
};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::setup_test_programs_with_accounts_v2,
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use light_sdk::{
    account_meta::InputAccountMeta,
    instruction_data::LightInstructionData,
    merkle_context::{pack_address_merkle_context, AddressMerkleContext, CpiAccounts},
    system_accounts::SystemAccountMetaConfig,
};
use sdk_test::{
    create_pda::CreatePdaInstructionData,
    update_pda::{UpdateMyCompressedAccount, UpdatePdaInstructionData},
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_sdk_test() {
    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![(String::from("sdk_test"), sdk_test::ID)]))
            .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::init_from_env(
        &payer,
        &env,
        // None,
        Some(ProverConfig {
            circuits: vec![ProofType::Inclusion, ProofType::NonInclusion],
            run_mode: None,
        }),
    )
    .await;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.batch_address_merkle_tree,
        address_queue_pubkey: env.batch_address_merkle_tree,
    };

    let account_data = [1u8; 31];

    // // V1 trees
    // let (address, _) = light_sdk::address::derive_address(
    //     &[b"compressed", &account_data],
    //     &address_merkle_context,
    //     &sdk_test::ID,
    // );
    // Batched trees
    let address_seed = hashv_to_bn254_field_size_be(&[b"compressed", account_data.as_slice()]);
    let address = derive_address(
        &address_seed,
        &address_merkle_context.address_merkle_tree_pubkey.to_bytes(),
        &sdk_test::ID.to_bytes(),
    );

    create_pda(
        &payer,
        &mut rpc,
        &mut test_indexer,
        &env.batched_output_queue,
        account_data,
        address_merkle_context,
        address,
    )
    .await
    .unwrap();

    let compressed_pda = test_indexer
        .get_compressed_accounts_by_owner_v2(&sdk_test::ID)
        .await
        .unwrap()[0]
        .clone();
    assert_eq!(compressed_pda.compressed_account.address.unwrap(), address);

    update_pda(
        &payer,
        &mut rpc,
        &mut test_indexer,
        [2u8; 31],
        compressed_pda,
        env.batched_output_queue,
    )
    .await
    .unwrap();
}

pub async fn create_pda(
    payer: &Keypair,
    rpc: &mut ProgramTestRpcConnection,
    test_indexer: &mut TestIndexer<ProgramTestRpcConnection>,
    merkle_tree_pubkey: &Pubkey,
    account_data: [u8; 31],
    address_merkle_context: AddressMerkleContext,
    address: [u8; 32],
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_test::ID);
    let mut accounts = CpiAccounts::default();
    accounts.insert_or_get_signer_mut(payer.pubkey());
    accounts.add_system_accounts(system_account_meta_config);

    let mut light_cpi_accounts = CpiAccounts::default();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[address]),
            Some(vec![address_merkle_context.address_merkle_tree_pubkey]),
            rpc,
        )
        .await
        .unwrap();

    let output_merkle_tree_index = light_cpi_accounts.insert_or_get(*merkle_tree_pubkey);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut light_cpi_accounts,
        rpc_result.address_root_indices[0],
    );
    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: Some(vec![packed_address_merkle_context]),
    };
    let instruction_data = CreatePdaInstructionData {
        light_ix_data,
        data: account_data,
        output_merkle_tree_index,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let system_accounts = accounts.to_account_metas();
    let light_cpi_accounts = light_cpi_accounts.to_account_metas();
    let accounts = [system_accounts, light_cpi_accounts].concat();
    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts,
        data: [&[0u8][..], &inputs[..]].concat(),
    };

    let (event, _, slot) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?
        .unwrap();
    test_indexer.add_event_and_compressed_accounts(slot, &event);
    Ok(())
}

pub async fn update_pda(
    payer: &Keypair,
    rpc: &mut ProgramTestRpcConnection,
    test_indexer: &mut TestIndexer<ProgramTestRpcConnection>,
    new_account_data: [u8; 31],
    compressed_account: CompressedAccountWithMerkleContext,
    output_merkle_tree: Pubkey,
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_test::ID);
    let mut accounts = CpiAccounts::default();
    accounts.insert_or_get_signer_mut(payer.pubkey());
    accounts.add_system_accounts(system_account_meta_config);

    let mut light_cpi_accounts = CpiAccounts::default();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts2(
            Some(vec![compressed_account.hash().unwrap()]),
            Some(vec![compressed_account.merkle_context.merkle_tree_pubkey]),
            None,
            None,
            rpc,
        )
        .await;

    let light_ix_data = LightInstructionData {
        proof: rpc_result.proof,
        new_addresses: None,
    };

    let instruction_data = UpdatePdaInstructionData {
        my_compressed_account: UpdateMyCompressedAccount {
            meta: InputAccountMeta::from_compressed_account(
                &compressed_account,
                &mut light_cpi_accounts,
                rpc_result.root_indices[0],
                &output_merkle_tree,
            )
            .unwrap(),
            data: compressed_account
                .compressed_account
                .data
                .unwrap()
                .data
                .try_into()
                .unwrap(),
        },
        light_ix_data,
        new_data: new_account_data,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let system_accounts = accounts.to_account_metas();
    let light_cpi_accounts = light_cpi_accounts.to_account_metas();
    let accounts = [system_accounts, light_cpi_accounts].concat();
    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts,
        data: [&[1u8][..], &inputs[..]].concat(),
    };

    let (event, _, slot) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event);
    Ok(())
}
