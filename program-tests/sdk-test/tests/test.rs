#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_compressed_account::{
    address::derive_address, compressed_account::CompressedAccountWithMerkleContext,
    hashv_to_bn254_field_size_be,
};
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, RpcConnection,
    RpcError,
};
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta,
    accounts::SystemAccountMetaConfig,
    merkle_context::{pack_address_merkle_context, AddressMerkleContext},
    pack_accounts::PackedAccounts,
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
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_test", sdk_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: rpc.get_address_merkle_tree_v2(),
        address_queue_pubkey: rpc.get_address_merkle_tree_v2(), // v2 queue is part of the tree account
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
    let ouput_queue = rpc.get_state_merkle_tree_v2().output_queue;
    create_pda(
        &payer,
        &mut rpc,
        &ouput_queue,
        account_data,
        address_merkle_context,
        address,
    )
    .await
    .unwrap();

    let compressed_pda = rpc
        .indexer()
        .unwrap()
        .get_compressed_accounts_by_owner(&sdk_test::ID)
        .await
        .unwrap()[0]
        .clone();
    assert_eq!(compressed_pda.compressed_account.address.unwrap(), address);

    update_pda(&payer, &mut rpc, [2u8; 31], compressed_pda, ouput_queue)
        .await
        .unwrap();
}

pub async fn create_pda(
    payer: &Keypair,
    rpc: &mut LightProgramTest,
    merkle_tree_pubkey: &Pubkey,
    account_data: [u8; 31],
    address_merkle_context: AddressMerkleContext,
    address: [u8; 32],
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_test::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts.add_system_accounts(system_account_meta_config);

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address,
                tree: address_merkle_context.address_merkle_tree_pubkey,
            }],
            None,
        )
        .await?;

    let output_merkle_tree_index = accounts.insert_or_get(*merkle_tree_pubkey);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut accounts,
        rpc_result.value.get_address_root_indices()[0],
    );
    let (accounts, system_accounts_offset, tree_accounts_offset) = accounts.to_account_metas();

    let instruction_data = CreatePdaInstructionData {
        proof: rpc_result.value.compressed_proof.0.unwrap().into(),
        address_merkle_context: packed_address_merkle_context,
        data: account_data,
        output_merkle_tree_index,
        system_accounts_offset: system_accounts_offset as u8,
        tree_accounts_offset: tree_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts,
        data: [&[0u8][..], &inputs[..]].concat(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;
    Ok(())
}

pub async fn update_pda(
    payer: &Keypair,
    rpc: &mut LightProgramTest,
    new_account_data: [u8; 31],
    compressed_account: CompressedAccountWithMerkleContext,
    output_merkle_tree: Pubkey,
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_test::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts.add_system_accounts(system_account_meta_config);

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], vec![], None)
        .await?;

    let meta = CompressedAccountMeta::from_compressed_account(
        &compressed_account,
        &mut accounts,
        rpc_result.value.get_root_indices()[0],
        &output_merkle_tree,
    )
    .unwrap();
    let (accounts, system_accounts_offset, _) = accounts.to_account_metas();
    let instruction_data = UpdatePdaInstructionData {
        my_compressed_account: UpdateMyCompressedAccount {
            meta,
            data: compressed_account
                .compressed_account
                .data
                .unwrap()
                .data
                .try_into()
                .unwrap(),
        },
        proof: rpc_result.value.compressed_proof,
        new_data: new_account_data,
        system_accounts_offset: system_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: sdk_test::ID,
        accounts,
        data: [&[1u8][..], &inputs[..]].concat(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;
    Ok(())
}
