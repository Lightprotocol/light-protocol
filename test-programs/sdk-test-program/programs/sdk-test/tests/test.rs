#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use forester_utils::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use light_client::rpc::merkle_tree::MerkleTreeExt;
use light_program_test::indexer::{TestIndexer, TestIndexerExtensions};
use light_program_test::test_env::{setup_test_programs_with_accounts_v2, EnvAccounts};
use light_program_test::test_rpc::ProgramTestRpcConnection;
use light_prover_client::gnark::helpers::{ProverConfig, ProverMode};
use light_sdk::account_meta::LightAccountMeta;
use light_sdk::address::derive_address;
use light_sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::instruction_data::LightInstructionData;
use light_sdk::merkle_context::{AddressMerkleContext, RemainingAccounts};
use light_sdk::proof::{CompressedProof, ProofRpcResult};
use light_sdk::utils::get_cpi_authority_pda;
use light_sdk::verify::find_cpi_signer;
use light_sdk::{PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_NOOP};
use light_test_utils::{RpcConnection, RpcError};
use sdk_test::{MyCompressedAccount, NestedData};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

#[tokio::test]
async fn test_sdk_test() {
    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![(String::from("sdk_test"), sdk_test::ID)]))
            .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::new(
        &[StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }],
        &[AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }],
        payer.insecure_clone(),
        env.group_pda,
        Some(ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }),
    )
    .await;

    let mut remaining_accounts = RemainingAccounts::default();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let (address, _) = derive_address(
        &[b"compressed", b"test"],
        &address_merkle_context,
        &sdk_test::ID,
    );

    let account_compression_authority = get_cpi_authority_pda(&PROGRAM_ID_LIGHT_SYSTEM);
    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;

    with_nested_data(
        "test".to_string(),
        &mut rpc,
        &mut test_indexer,
        &env,
        &mut remaining_accounts,
        &payer,
        &address,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was created correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&sdk_test::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = MyCompressedAccount::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.nested.one, 1);

    update_nested_data(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        NestedData {
            one: 2,
            two: 3,
            three: 3,
            four: 4,
            five: 5,
            six: 6,
            seven: 7,
            eight: 8,
            nine: 9,
            ten: 10,
            eleven: 11,
            twelve: 12,
        },
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was updated correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&sdk_test::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = MyCompressedAccount::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.nested.one, 2);
}

#[allow(clippy::too_many_arguments)]
async fn with_nested_data<R>(
    name: String,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    address: &[u8; 32],
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            rpc,
        )
        .await;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };
    let account = LightAccountMeta::new_init(
        &env.merkle_tree_pubkey,
        Some(&address_merkle_context),
        Some(rpc_result.address_root_indices[0]),
        remaining_accounts,
    )
    .unwrap();

    let proof_result = ProofRpcResult {
        root_indices: rpc_result.address_root_indices.clone(),
        address_root_indices: rpc_result.address_root_indices.clone(),
        proof: CompressedProof {
            a: rpc_result.proof.a,
            b: rpc_result.proof.b,
            c: rpc_result.proof.c,
        },
    };

    let inputs = LightInstructionData {
        proof: Some(proof_result),
        accounts: Some(vec![account]),
    };
    let inputs = inputs.serialize().unwrap();

    let instruction_data = sdk_test::instruction::WithNestedData { inputs, name };

    let cpi_signer = find_cpi_signer(&sdk_test::ID);

    let accounts = sdk_test::accounts::WithNestedData {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: sdk_test::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    test_indexer.add_compressed_accounts_with_token_data(&event.unwrap().0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_nested_data<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut RemainingAccounts,
    nested_data: NestedData,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[hash]),
            Some(&[merkle_tree_pubkey]),
            None,
            None,
            rpc,
        )
        .await;

    let compressed_account = LightAccountMeta::new_mut(
        compressed_account,
        rpc_result.root_indices[0],
        &merkle_tree_pubkey,
        remaining_accounts,
    );

    let proof_result = ProofRpcResult {
        root_indices: rpc_result.address_root_indices.clone(),
        address_root_indices: rpc_result.address_root_indices.clone(),
        proof: CompressedProof {
            a: rpc_result.proof.a,
            b: rpc_result.proof.b,
            c: rpc_result.proof.c,
        },
    };

    let inputs = LightInstructionData {
        proof: Some(proof_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = sdk_test::instruction::UpdateNestedData {
        inputs,
        nested_data,
    };

    let cpi_signer = find_cpi_signer(&sdk_test::ID);

    let accounts = sdk_test::accounts::UpdateNestedData {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: sdk_test::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    test_indexer.add_compressed_accounts_with_token_data(&event.unwrap().0);
    Ok(())
}
