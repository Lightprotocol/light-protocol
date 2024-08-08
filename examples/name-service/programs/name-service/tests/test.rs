#![cfg(feature = "test-sbf")]

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_system_program::sdk::address::{derive_address, pack_new_address_params};
use light_system_program::sdk::compressed_account::{
    CompressedAccountWithMerkleContext, PackedCompressedAccountWithMerkleContext,
    PackedMerkleContext,
};
use light_system_program::NewAddressParams;
use light_test_utils::indexer::{test_indexer::TestIndexer, Indexer};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use name_service::{NameRecord, RData};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

fn find_cpi_signer() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"cpi_signer"], &name_service::ID)
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

    let seed = [1u8; 32];
    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();

    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;

    let rdata_1 = RData::A(Ipv4Addr::new(10, 0, 1, 25));

    create_record(
        &mut rpc,
        &mut test_indexer,
        &env,
        &rdata_1,
        &payer,
        &seed,
        &address,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;

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

    let rdata_2 = RData::AAAA(Ipv6Addr::new(8193, 3512, 0, 0, 0, 0, 0, 1));

    update_record(
        &mut rpc,
        &mut test_indexer,
        &env,
        &rdata_2,
        &payer,
        compressed_account,
        &address,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;

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

    delete_record(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
    )
    .await;
}

async fn create_record<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    rdata: &RData,
    payer: &Keypair,
    seed: &[u8; 32],
    address: &[u8; 32],
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

    let new_address_params = NewAddressParams {
        seed: *seed,
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
    };

    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(env.merkle_tree_pubkey, 0);
    remaining_accounts.insert(env.nullifier_queue_pubkey, 1);
    remaining_accounts.insert(env.cpi_context_account_pubkey, 2);

    let new_address_params =
        pack_new_address_params(&[new_address_params], &mut remaining_accounts)[0];

    let instruction_data = name_service::instruction::CreateRecord {
        proof: rpc_result.proof,
        new_address_params,
        name: "example.io".to_string(),
        rdata: rdata.clone(),
        // cpi_context: Some(CompressedCpiContext {
        //     cpi_context_account_index: 2,
        //     set_context: true,
        //     first_set_context: true,
        // }),
        cpi_context: None,
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = instruction_accounts(
        payer,
        account_compression_authority,
        registered_program_pda,
        &cpi_signer,
    );
    let remaining_accounts = to_account_metas(remaining_accounts);

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
    rdata: &RData,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    address: &[u8; 32],
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

    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(env.merkle_tree_pubkey, 0);
    remaining_accounts.insert(env.nullifier_queue_pubkey, 1);
    remaining_accounts.insert(env.cpi_context_account_pubkey, 2);

    let instruction_data = name_service::instruction::UpdateRecord {
        compressed_account: PackedCompressedAccountWithMerkleContext {
            compressed_account: compressed_account.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                leaf_index: compressed_account.merkle_context.leaf_index,
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 1,
                queue_index: None,
            },
            root_index: rpc_result.root_indices[0],
        },
        proof: rpc_result.proof,
        address: *address,
        name: "example.io".to_string(),
        rdata: rdata.clone(),
        // cpi_context: Some(CompressedCpiContext {
        //     cpi_context_account_index: 2,
        //     set_context: true,
        //     first_set_context: true,
        // }),
        cpi_context: None,
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = instruction_accounts(
        payer,
        account_compression_authority,
        registered_program_pda,
        &cpi_signer,
    );
    let remaining_accounts = to_account_metas(remaining_accounts);

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
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
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

    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(env.merkle_tree_pubkey, 0);
    remaining_accounts.insert(env.nullifier_queue_pubkey, 1);
    remaining_accounts.insert(env.cpi_context_account_pubkey, 2);

    let instruction_data = name_service::instruction::DeleteRecord {
        compressed_account: PackedCompressedAccountWithMerkleContext {
            compressed_account: compressed_account.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                leaf_index: compressed_account.merkle_context.leaf_index,
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 1,
                queue_index: None,
            },
            root_index: rpc_result.root_indices[0],
        },
        proof: rpc_result.proof,
        // cpi_context: Some(CompressedCpiContext {
        //     cpi_context_account_index: 2,
        //     set_context: true,
        //     first_set_context: true,
        // }),
        cpi_context: None,
    };

    let (cpi_signer, _) = find_cpi_signer();

    let accounts = instruction_accounts(
        payer,
        account_compression_authority,
        registered_program_pda,
        &cpi_signer,
    );
    let remaining_accounts = to_account_metas(remaining_accounts);

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

fn instruction_accounts(
    payer: &Keypair,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    cpi_signer: &Pubkey,
) -> name_service::accounts::NameService {
    name_service::accounts::NameService {
        signer: payer.pubkey(),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        self_program: name_service::ID,
        cpi_signer: *cpi_signer,
        system_program: solana_sdk::system_program::id(),
    }
}
