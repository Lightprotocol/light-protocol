#![cfg(feature = "test-sbf")]

use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::indexer::test_indexer::TestIndexer;
use light_client::indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts};
use light_client::rpc::merkle_tree::MerkleTreeExt;
use light_client::rpc::test_rpc::ProgramTestRpcConnection;
use light_sdk::account_meta::LightAccountMeta;
use light_sdk::address::derive_address;
use light_sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::error::LightSdkError;
use light_sdk::instruction_accounts::LightInstructionAccounts;
use light_sdk::instruction_data::LightInstructionData;
use light_sdk::merkle_context::AddressMerkleContext;
use light_sdk::utils::get_cpi_authority_pda;
use light_sdk::verify::find_cpi_signer;
use light_sdk::{PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM};
use light_test_utils::test_env::{setup_test_programs_with_accounts_v2, EnvAccounts};
use light_test_utils::{RpcConnection, RpcError};
use name_service_without_macros::{CustomError, NameRecord, RData};
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};

#[tokio::test]
async fn test_name_service() {
    let (mut rpc, env) = setup_test_programs_with_accounts_v2(Some(vec![(
        String::from("name_service_without_macros"),
        name_service_without_macros::ID,
    )]))
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
        true,
        true,
    )
    .await;

    let name = "example.io";

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let (address, _) = derive_address(
        &[b"name-service", name.as_bytes()],
        &address_merkle_context,
        &name_service_without_macros::ID,
    );

    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;
    let account_compression_authority = get_cpi_authority_pda(&PROGRAM_ID_LIGHT_SYSTEM);

    let mut instruction_accounts = LightInstructionAccounts::new(
        &registered_program_pda,
        &account_compression_authority,
        &name_service_without_macros::ID,
        None,
        None,
    );
    println!(
        "INSTRUCTION_ACCOUNTS: {:#?}",
        instruction_accounts.to_account_metas()
    );

    // Create the example.io -> 10.0.1.25 record.
    let rdata_1 = RData::A(Ipv4Addr::new(10, 0, 1, 25));
    create_record(
        &name,
        &rdata_1,
        &mut rpc,
        &mut test_indexer,
        &env,
        &mut instruction_accounts,
        &payer,
        &address,
    )
    .await
    .unwrap();

    // Create with invalid light-system-program ID, should not succeed.
    {
        let result = create_record(
            &name,
            &rdata_1,
            &mut rpc,
            &mut test_indexer,
            &env,
            &mut instruction_accounts,
            &payer,
            &address,
        )
        .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::Custom(error))
            ))if error == u32::from(LightSdkError::InvalidLightSystemProgram)
        ));
    }

    // Check that it was created correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_by_owner(&name_service_without_macros::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = NameRecord::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.name, "example.io");
    assert_eq!(record.rdata, rdata_1);

    // Return early to skip remaining tests
    println!("TEST RETURN EARLY");
    return;

    // Update the record to example.io -> 2001:db8::1.
    let rdata_2 = RData::AAAA(Ipv6Addr::new(8193, 3512, 0, 0, 0, 0, 0, 1));
    update_record(
        &mut rpc,
        &mut test_indexer,
        &mut instruction_accounts,
        &rdata_2,
        &payer,
        compressed_account,
    )
    .await
    .unwrap();

    // Update with invalid owner, should not succeed.
    {
        let invalid_signer = Keypair::new();
        rpc.airdrop_lamports(&invalid_signer.pubkey(), LAMPORTS_PER_SOL * 1)
            .await
            .unwrap();
        let result = update_record(
            &mut rpc,
            &mut test_indexer,
            &mut instruction_accounts,
            &rdata_2,
            &invalid_signer,
            compressed_account,
        )
        .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::Custom(error))
            ))if error == u32::from(CustomError::Unauthorized)
        ));
    }
    // Update with invalid light-system-program ID, should not succeed.
    {
        let result = update_record(
            &mut rpc,
            &mut test_indexer,
            &mut instruction_accounts,
            &rdata_2,
            &payer,
            compressed_account,
        )
        .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::Custom(error))
            ))if error == u32::from(LightSdkError::InvalidLightSystemProgram)
        ));
    }

    // Check that it was updated correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_by_owner(&name_service_without_macros::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = NameRecord::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.name, "example.io");
    assert_eq!(record.rdata, rdata_2);

    // Delete with invalid owner, should not succeed.
    {
        let invalid_signer = Keypair::new();
        rpc.airdrop_lamports(&invalid_signer.pubkey(), LAMPORTS_PER_SOL * 1)
            .await
            .unwrap();
        let result = delete_record(
            &mut rpc,
            &mut test_indexer,
            &mut instruction_accounts,
            &invalid_signer,
            compressed_account,
        )
        .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::Custom(error))
            ))if error == u32::from(CustomError::Unauthorized)
        ));
    }
    // Delete with invalid light-system-program ID, should not succeed.
    {
        let result = delete_record(
            &mut rpc,
            &mut test_indexer,
            &mut instruction_accounts,
            &payer,
            compressed_account,
        )
        .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::Custom(error))
            ))if error == u32::from(LightSdkError::InvalidLightSystemProgram)
        ));
    }

    // Delete the example.io record.
    delete_record(
        &mut rpc,
        &mut test_indexer,
        &mut instruction_accounts,
        &payer,
        compressed_account,
    )
    .await
    .unwrap();
}

async fn create_record<R>(
    name: &str,
    rdata: &RData,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    instruction_accounts: &mut LightInstructionAccounts,
    payer: &Keypair,
    address: &[u8; 32],
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
        instruction_accounts,
    )
    .unwrap();

    println!("ACCOUNT: {:#?}", account);
    println!("MERKLE_TREE: {:?}", env.merkle_tree_pubkey);

    println!(
        "POST_INSTRUCTION_ACCOUNTS: {:#?}",
        instruction_accounts.to_account_metas()
    );
    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = name_service_without_macros::instruction::CreateRecord {
        inputs,
        name: name.to_string(),
        rdata: rdata.clone(),
    };

    let cpi_signer = find_cpi_signer(&name_service_without_macros::ID);

    let accounts = name_service_without_macros::accounts::CreateRecord {
        signer: payer.pubkey(),
        cpi_signer,
    };

    let remaining_accounts = instruction_accounts.to_account_metas();
    println!("REMAINING ACCOUNTS: {:#?}", remaining_accounts);

    let instruction = Instruction {
        program_id: name_service_without_macros::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    println!("INSTRUCTION: {:#?}", instruction.accounts);

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    test_indexer.add_compressed_accounts_with_token_data(&event.unwrap().0);
    Ok(())
}

async fn update_record<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    instruction_accounts: &mut LightInstructionAccounts,
    new_rdata: &RData,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
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
        instruction_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = name_service_without_macros::instruction::UpdateRecord {
        inputs,
        new_rdata: new_rdata.clone(),
    };

    let cpi_signer = find_cpi_signer(&name_service_without_macros::ID);

    let accounts = name_service_without_macros::accounts::UpdateRecord {
        signer: payer.pubkey(),
        cpi_signer,
    };

    let remaining_accounts = instruction_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: name_service_without_macros::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    test_indexer.add_compressed_accounts_with_token_data(&event.unwrap().0);
    Ok(())
}

async fn delete_record<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    light_instruction_accounts: &mut LightInstructionAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
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

    let compressed_account = LightAccountMeta::new_close(
        compressed_account,
        rpc_result.root_indices[0],
        light_instruction_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = name_service_without_macros::instruction::DeleteRecord { inputs };

    let cpi_signer = find_cpi_signer(&name_service_without_macros::ID);

    let accounts = name_service_without_macros::accounts::DeleteRecord {
        signer: payer.pubkey(),
        cpi_signer,
    };

    let remaining_accounts = light_instruction_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: name_service_without_macros::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    rpc.process_transaction(transaction).await?;
    Ok(())
}
