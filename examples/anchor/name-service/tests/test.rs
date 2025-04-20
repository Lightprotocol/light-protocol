#![cfg(feature = "test-sbf")]

use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::merkle_tree::MerkleTreeExt,
};
use light_program_test::{
    test_env::{setup_test_programs_with_accounts_v2, EnvAccounts},
    test_indexer::TestIndexer,
    test_rpc::ProgramTestRpcConnection,
};
use light_sdk::{
    address::{derive_address, derive_address_seed},
    compressed_account::CompressedAccountWithMerkleContext,
    error::LightSdkError,
    merkle_context::{
        pack_address_merkle_context, pack_merkle_context, AddressMerkleContext, MerkleContext,
        PackedAddressMerkleContext, PackedMerkleContext, PackedAccounts,
    },
    utils::get_cpi_authority_pda,
    verify::find_cpi_signer,
    PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_NOOP,
};
use light_test_utils::{RpcConnection, RpcError};
use name_service::{CustomError, NameRecord, RData};
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};

#[tokio::test]
async fn test_name_service() {
    let (mut rpc, env) = setup_test_programs_with_accounts_v2(Some(vec![(
        String::from("name_service"),
        name_service::ID,
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

    let mut remaining_accounts = PackedAccounts::default();

    let merkle_context = MerkleContext {
        merkle_tree_pubkey: env.merkle_tree_pubkey,
        nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        leaf_index: 0,
        prove_by_index: false,
        tree_type: TreeType::StateV1
    };
    let merkle_context = pack_merkle_context(&merkle_context, &mut remaining_accounts);

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let address_seed =
        derive_address_legacy_seed(&[b"name-service", name.as_bytes()], &name_service::ID);
    let address = derive_address_legacy(&address_seed, &address_merkle_context);

    let address_merkle_context =
        pack_address_merkle_context(&address_merkle_context, &mut remaining_accounts);

    let account_compression_authority = get_cpi_authority_pda(&PROGRAM_ID_LIGHT_SYSTEM);
    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;

    // Create the example.io -> 10.0.1.25 record.
    let rdata_1 = RData::A(Ipv4Addr::new(10, 0, 1, 25));
    create_record(
        &name,
        &rdata_1,
        &mut rpc,
        &mut test_indexer,
        &env,
        &mut remaining_accounts,
        &payer,
        &address,
        &merkle_context,
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
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
            &mut remaining_accounts,
            &payer,
            &address,
            &merkle_context,
            &address_merkle_context,
            &account_compression_authority,
            &registered_program_pda,
            &Pubkey::new_unique(),
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
    let compressed_accounts = test_indexer.get_compressed_accounts_by_owner_v2(&name_service::ID);
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

    // Update the record to example.io -> 2001:db8::1.
    let rdata_2 = RData::AAAA(Ipv6Addr::new(8193, 3512, 0, 0, 0, 0, 0, 1));
    update_record(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &rdata_2,
        &payer,
        compressed_account,
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
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
            &mut remaining_accounts,
            &rdata_2,
            &invalid_signer,
            compressed_account,
            &address_merkle_context,
            &account_compression_authority,
            &registered_program_pda,
            &PROGRAM_ID_LIGHT_SYSTEM,
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
            &mut remaining_accounts,
            &rdata_2,
            &payer,
            compressed_account,
            &address_merkle_context,
            &account_compression_authority,
            &registered_program_pda,
            &Pubkey::new_unique(),
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
    let compressed_accounts = test_indexer.get_compressed_accounts_by_owner_v2(&name_service::ID);
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
            &mut remaining_accounts,
            &invalid_signer,
            compressed_account,
            &address_merkle_context,
            &account_compression_authority,
            &registered_program_pda,
            &PROGRAM_ID_LIGHT_SYSTEM,
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
            &mut remaining_accounts,
            &payer,
            compressed_account,
            &address_merkle_context,
            &account_compression_authority,
            &registered_program_pda,
            &Pubkey::new_unique(),
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
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
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
    remaining_accounts: &mut PackedAccounts,
    payer: &Keypair,
    address: &[u8; 32],
    merkle_context: &PackedMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
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

    let instruction_data = name_service::instruction::CreateRecord {
        inputs: Vec::new(),
        proof: rpc_result.proof,
        merkle_context: *merkle_context,
        merkle_tree_root_index: 0,
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        name: name.to_string(),
        rdata: rdata.clone(),
    };

    let cpi_signer = find_cpi_signer(&name_service::ID);

    let accounts = name_service::accounts::CreateRecord {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: name_service::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: name_service::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_public_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

async fn update_record<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut PackedAccounts,
    new_rdata: &RData,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
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

    let merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);

    let inputs = vec![
        compressed_account
            .compressed_account
            .data
            .clone()
            .unwrap()
            .data,
    ];

    let instruction_data = name_service::instruction::UpdateRecord {
        inputs,
        proof: rpc_result.proof,
        merkle_context,
        merkle_tree_root_index: rpc_result.root_indices[0].unwrap(),
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: 0,
        new_rdata: new_rdata.clone(),
    };

    let cpi_signer = find_cpi_signer(&name_service::ID);

    let accounts = name_service::accounts::UpdateRecord {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: name_service::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: name_service::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_public_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

async fn delete_record<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut PackedAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
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

    let merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, remaining_accounts);

    let inputs = vec![
        compressed_account
            .compressed_account
            .data
            .clone()
            .unwrap()
            .data,
    ];

    let instruction_data = name_service::instruction::DeleteRecord {
        inputs,
        proof: rpc_result.proof,
        merkle_context,
        merkle_tree_root_index: rpc_result.root_indices[0].unwrap(),
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: 0,
    };

    let cpi_signer = find_cpi_signer(&name_service::ID);

    let accounts = name_service::accounts::DeleteRecord {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: name_service::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: name_service::ID,
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
