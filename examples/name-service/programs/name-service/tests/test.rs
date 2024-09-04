#![cfg(feature = "test-sbf")]

use std::net::{Ipv4Addr, Ipv6Addr};

use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_sdk::address::derive_address_seed;
use light_sdk::merkle_context::{
    pack_address_merkle_context, pack_merkle_context, pack_merkle_output_context,
    AddressMerkleContext, MerkleContext, MerkleOutputContext, PackedAddressMerkleContext,
    RemainingAccounts,
};
use light_system_program::sdk::address::derive_address;
use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_test_utils::indexer::test_indexer::TestIndexer;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::{Indexer, RpcConnection};
use name_service::{NameRecord, RData};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

fn find_cpi_signer() -> (Pubkey, u8) {
    Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), &name_service::ID)
}

#[tokio::test]
async fn test_name_service() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("name_service"),
        name_service::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> =
        TestIndexer::init_from_env(&payer, &env, true, true).await;

    let name = "example.io";

    let mut remaining_accounts = RemainingAccounts::default();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let address_seed = derive_address_seed(
        &[b"name-service", name.as_bytes()],
        &name_service::ID,
        &address_merkle_context,
    );
    let address = derive_address(&env.address_merkle_tree_pubkey, &address_seed).unwrap();

    let address_merkle_context =
        pack_address_merkle_context(address_merkle_context, &mut remaining_accounts);

    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
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
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;

    // Check that it was created correctly.
    let compressed_accounts = test_indexer.get_compressed_accounts_by_owner(&name_service::ID);
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
        &env,
        &mut remaining_accounts,
        &rdata_2,
        &payer,
        compressed_account,
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;

    // Check that it was updated correctly.
    let compressed_accounts = test_indexer.get_compressed_accounts_by_owner(&name_service::ID);
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

    // Delete the example.io record.
    delete_record(
        &mut rpc,
        &mut test_indexer,
        &env,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &address_merkle_context,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;
}

async fn create_record<R: RpcConnection>(
    name: &str,
    rdata: &RData,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    address: &[u8; 32],
    address_merkle_context: &PackedAddressMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
) {
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            rpc,
        )
        .await;

    // Input Merkle tree.
    let input_merkle_context = MerkleContext {
        merkle_tree_pubkey: env.merkle_tree_pubkey,
        nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        leaf_index: 0,
        queue_index: None,
    };
    let input_merkle_context = pack_merkle_context(input_merkle_context, remaining_accounts);

    // Output Merkle tree.
    let output_merkle_context = MerkleOutputContext {
        merkle_tree_pubkey: env.merkle_tree_pubkey,
    };
    let output_merkle_context =
        pack_merkle_output_context(output_merkle_context, remaining_accounts);

    let instruction_data = name_service::instruction::CreateRecord {
        inputs: Vec::new(),
        proof: rpc_result.proof,
        input_merkle_context,
        input_merkle_tree_root_index: 0,
        output_merkle_context,
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        name: name.to_string(),
        rdata: rdata.clone(),
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = name_service::accounts::CreateRecord {
        signer: payer.pubkey(),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
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
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await
        .unwrap()
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(&event.0);
}

async fn update_record<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    remaining_accounts: &mut RemainingAccounts,
    new_rdata: &RData,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
) {
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

    // Input Merkle tree.
    let input_merkle_context =
        pack_merkle_context(compressed_account.merkle_context, remaining_accounts);

    // Output Merkle tree.
    let output_merkle_context = MerkleOutputContext {
        merkle_tree_pubkey: env.merkle_tree_pubkey,
    };
    let output_merkle_context =
        pack_merkle_output_context(output_merkle_context, remaining_accounts);

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
        input_merkle_context,
        input_merkle_tree_root_index: rpc_result.root_indices[0],
        output_merkle_context,
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: 0,
        new_rdata: new_rdata.clone(),
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = name_service::accounts::UpdateRecord {
        signer: payer.pubkey(),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
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
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await
        .unwrap()
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(&event.0);
}

async fn delete_record<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
) {
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

    // Input Merkle tree.
    let input_merkle_context =
        pack_merkle_context(compressed_account.merkle_context, remaining_accounts);

    // Output Merkle tree.
    let output_merkle_context = MerkleOutputContext {
        merkle_tree_pubkey: env.merkle_tree_pubkey,
    };
    let output_merkle_context =
        pack_merkle_output_context(output_merkle_context, remaining_accounts);

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
        input_merkle_context,
        input_merkle_tree_root_index: rpc_result.root_indices[0],
        output_merkle_context,
        address_merkle_context: *address_merkle_context,
        address_merkle_tree_root_index: 0,
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = name_service::accounts::DeleteRecord {
        signer: payer.pubkey(),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
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
    rpc.process_transaction(transaction).await.unwrap();
}
