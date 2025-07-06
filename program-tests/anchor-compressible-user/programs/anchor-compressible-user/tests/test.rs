#![cfg(feature = "test-sbf")]

use anchor_compressible_user::{UserRecord, UserRecordCreated};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_account::{
    address::derive_address, compressed_account::CompressedAccountWithMerkleContext,
    hashv_to_bn254_field_size_be,
};
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_anchor_compressible_user() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("anchor_compressible_user", anchor_compressible_user::ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    
    // Test 1: Create a user record
    let user_name = "Alice".to_string();
    let user_bio = "I love compressed accounts!".to_string();
    
    let address = create_user_record(
        &mut rpc,
        &payer,
        user_name.clone(),
        user_bio.clone(),
    )
    .await
    .unwrap();
    
    // Verify the account was created
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .clone();
    
    assert_eq!(compressed_account.address.unwrap(), address);
    
    // Test 2: Update the user record
    let new_bio = "I REALLY love compressed accounts!".to_string();
    update_user_record(
        &mut rpc,
        &payer,
        compressed_account.into(),
        None,
        Some(new_bio.clone()),
        Some(100),
    )
    .await
    .unwrap();
    
    // Test 3: Decompress the user record
    let compressed_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .clone();
    
    decompress_user_record(&mut rpc, &payer, compressed_account.into())
        .await
        .unwrap();
    
    // Verify the PDA was created
    let pda = Pubkey::find_program_address(
        &[b"user_record", payer.pubkey().as_ref()],
        &anchor_compressible_user::ID,
    )
    .0;
    
    let pda_account = rpc.get_account(pda).await.unwrap();
    assert!(pda_account.is_some());
}

async fn create_user_record(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    name: String,
    bio: String,
) -> Result<[u8; 32], RpcError> {
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;
    
    // Derive the address based on user's pubkey
    let address_seed = hashv_to_bn254_field_size_be(&[b"user_record", user.pubkey().as_ref()]);
    let address = derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &anchor_compressible_user::ID.to_bytes(),
    );
    
    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await?
        .value;
    
    // Pack accounts
    let system_account_meta_config = SystemAccountMetaConfig::new(anchor_compressible_user::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(user.pubkey());
    accounts.add_system_accounts(system_account_meta_config);
    
    let output_merkle_tree_index = accounts.insert_or_get(output_queue);
    let packed_address_tree_info = rpc_result.pack_tree_infos(&mut accounts).address_trees[0];
    let (accounts, _, _) = accounts.to_account_metas();
    
    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::CreateUserRecord {
        proof: rpc_result.proof,
        address_tree_info: packed_address_tree_info,
        output_tree_index: output_merkle_tree_index,
        name,
        bio,
    };
    
    let instruction = Instruction {
        program_id: anchor_compressible_user::ID,
        accounts,
        data: instruction_data.data(),
    };
    
    rpc.create_and_send_transaction(&[instruction], &user.pubkey(), &[user])
        .await?;
    
    Ok(address)
}

async fn update_user_record(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    compressed_account: CompressedAccountWithMerkleContext,
    new_name: Option<String>,
    new_bio: Option<String>,
    score_delta: Option<i64>,
) -> Result<(), RpcError> {
    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], vec![], None)
        .await?
        .value;
    
    // Pack accounts
    let system_account_meta_config = SystemAccountMetaConfig::new(anchor_compressible_user::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(user.pubkey());
    accounts.add_system_accounts(system_account_meta_config);
    
    let packed_accounts = rpc_result
        .pack_tree_infos(&mut accounts)
        .state_trees
        .unwrap();
    
    let meta = CompressedAccountMeta {
        tree_info: packed_accounts.packed_tree_infos[0],
        address: compressed_account.compressed_account.address.unwrap(),
        output_state_tree_index: packed_accounts.output_tree_index,
    };
    
    let (accounts, _, _) = accounts.to_account_metas();
    
    // Deserialize current record
    let current_record: UserRecord = UserRecord::deserialize(
        &mut &compressed_account
            .compressed_account
            .data
            .unwrap()
            .data[..],
    )
    .unwrap();
    
    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::UpdateUserRecord {
        proof: rpc_result.proof,
        account_meta: meta,
        current_record,
        new_name,
        new_bio,
        score_delta,
    };
    
    let instruction = Instruction {
        program_id: anchor_compressible_user::ID,
        accounts,
        data: instruction_data.data(),
    };
    
    rpc.create_and_send_transaction(&[instruction], &user.pubkey(), &[user])
        .await?;
    
    Ok(())
}

async fn decompress_user_record(
    rpc: &mut LightProgramTest,
    user: &Keypair,
    compressed_account: CompressedAccountWithMerkleContext,
) -> Result<(), RpcError> {
    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], vec![], None)
        .await?
        .value;
    
    // Pack accounts
    let system_account_meta_config = SystemAccountMetaConfig::new(anchor_compressible_user::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(user.pubkey());
    accounts.add_system_accounts(system_account_meta_config);
    
    let packed_accounts = rpc_result
        .pack_tree_infos(&mut accounts)
        .state_trees
        .unwrap();
    
    let meta = CompressedAccountMeta {
        tree_info: packed_accounts.packed_tree_infos[0],
        address: compressed_account.compressed_account.address.unwrap(),
        output_state_tree_index: packed_accounts.output_tree_index,
    };
    
    // Deserialize current record
    let compressed_record: UserRecord = UserRecord::deserialize(
        &mut &compressed_account
            .compressed_account
            .data
            .unwrap()
            .data[..],
    )
    .unwrap();
    
    // Get the PDA account
    let user_record_pda = Pubkey::find_program_address(
        &[b"user_record", user.pubkey().as_ref()],
        &anchor_compressible_user::ID,
    )
    .0;
    
    // Create instruction accounts
    let instruction_accounts = anchor_compressible_user::accounts::DecompressUserRecord {
        user: user.pubkey(),
        user_record_pda,
        system_program: solana_sdk::system_program::ID,
    };
    
    let (mut accounts, _, _) = accounts.to_account_metas();
    accounts.extend_from_slice(&instruction_accounts.to_account_metas(Some(true)));
    
    // Create instruction data
    let instruction_data = anchor_compressible_user::instruction::DecompressUserRecord {
        proof: rpc_result.proof,
        account_meta: meta,
        compressed_record,
    };
    
    let instruction = Instruction {
        program_id: anchor_compressible_user::ID,
        accounts,
        data: instruction_data.data(),
    };
    
    rpc.create_and_send_transaction(&[instruction], &user.pubkey(), &[user])
        .await?;
    
    Ok(())
} 