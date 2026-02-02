#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_compressed_account::{
    address::derive_address, compressed_account::CompressedAccountWithMerkleContext,
    hashv_to_bn254_field_size_be,
};
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use light_sdk_pinocchio::instruction::{account_meta::CompressedAccountMeta, PackedStateTreeInfo};
use sdk_pinocchio_v2_test::{
    create_pda::CreatePdaInstructionData,
    update_pda::{UpdateMyCompressedAccount, UpdatePdaInstructionData},
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_pinocchio_sdk_test() {
    let config = ProgramTestConfig::new_v2(
        false,
        Some(vec![(
            "sdk_pinocchio_v2_test",
            Pubkey::new_from_array(sdk_pinocchio_v2_test::ID),
        )]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_pubkey = rpc.get_address_tree_v2();
    let account_data = [1u8; 31];

    // // V1 trees
    // let (address, _) = light_sdk::address::derive_address(
    //     &[b"compressed", &account_data],
    //     &address_tree_info,
    //     &Pubkey::new_from_array(sdk_pinocchio_v2_test::ID),
    // );
    // Batched trees
    let address_seed = hashv_to_bn254_field_size_be(&[b"compressed", account_data.as_slice()]);
    let address = derive_address(
        &address_seed,
        &address_tree_pubkey.tree.to_bytes(),
        &sdk_pinocchio_v2_test::ID,
    );

    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    create_pda(
        &payer,
        &mut rpc,
        &output_queue,
        account_data,
        address_tree_pubkey.tree,
        address,
    )
    .await
    .unwrap();

    let compressed_pda = rpc
        .indexer()
        .unwrap()
        .get_compressed_accounts_by_owner(
            &Pubkey::new_from_array(sdk_pinocchio_v2_test::ID),
            None,
            None,
        )
        .await
        .unwrap()
        .value
        .items[0]
        .clone();
    assert_eq!(compressed_pda.address.unwrap(), address);

    update_pda(&payer, &mut rpc, [2u8; 31], compressed_pda.into())
        .await
        .unwrap();
}

pub async fn create_pda(
    payer: &Keypair,
    rpc: &mut LightProgramTest,
    merkle_tree_pubkey: &Pubkey,
    account_data: [u8; 31],
    address_tree_pubkey: Pubkey,
    address: [u8; 32],
) -> Result<(), RpcError> {
    let system_account_meta_config =
        SystemAccountMetaConfig::new(Pubkey::new_from_array(sdk_pinocchio_v2_test::ID));
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts
        .add_system_accounts_v2(system_account_meta_config)
        .unwrap();

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

    let output_merkle_tree_index = accounts.insert_or_get(*merkle_tree_pubkey);
    let packed_address_tree_info = rpc_result.pack_tree_infos(&mut accounts).address_trees[0];
    let (accounts, system_accounts_offset, tree_accounts_offset) = accounts.to_account_metas();
    let instruction_data = CreatePdaInstructionData {
        proof: rpc_result.proof,
        address_tree_info: packed_address_tree_info,
        data: account_data,
        output_merkle_tree_index,
        system_accounts_offset: system_accounts_offset as u8,
        tree_accounts_offset: tree_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: Pubkey::new_from_array(sdk_pinocchio_v2_test::ID),
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
) -> Result<(), RpcError> {
    let system_account_meta_config =
        SystemAccountMetaConfig::new(Pubkey::new_from_array(sdk_pinocchio_v2_test::ID));
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts
        .add_system_accounts_v2(system_account_meta_config)
        .unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], vec![], None)
        .await?
        .value;

    let packed_accounts = rpc_result
        .pack_tree_infos(&mut accounts)
        .state_trees
        .unwrap();

    let light_sdk_meta = CompressedAccountMeta {
        tree_info: packed_accounts.packed_tree_infos[0],
        address: compressed_account.compressed_account.address.unwrap(),
        output_state_tree_index: packed_accounts.output_tree_index,
    };

    // Convert to pinocchio CompressedAccountMeta
    let meta = CompressedAccountMeta {
        tree_info: PackedStateTreeInfo {
            root_index: light_sdk_meta.tree_info.root_index,
            prove_by_index: light_sdk_meta.tree_info.prove_by_index,
            merkle_tree_pubkey_index: light_sdk_meta.tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: light_sdk_meta.tree_info.queue_pubkey_index,
            leaf_index: light_sdk_meta.tree_info.leaf_index,
        },
        address: light_sdk_meta.address,
        output_state_tree_index: light_sdk_meta.output_state_tree_index,
    };

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
        proof: light_sdk_pinocchio::instruction::ValidityProof(None),
        new_data: new_account_data,
        system_accounts_offset: system_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: Pubkey::new_from_array(sdk_pinocchio_v2_test::ID),
        accounts,
        data: [&[1u8][..], &inputs[..]].concat(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;
    Ok(())
}
