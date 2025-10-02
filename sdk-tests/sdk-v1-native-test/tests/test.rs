//#![cfg(feature = "test-sbf")]

use borsh::BorshSerialize;
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig,
};
use sdk_native_test::{
    create_pda::CreatePdaInstructionData,
    update_pda::{UpdateMyCompressedAccount, UpdatePdaInstructionData},
    ARRAY_LEN,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_sdk_native_test() {
    let config = ProgramTestConfig::new(true, Some(vec![("sdk_native_test", sdk_native_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree = rpc.get_address_tree_v1().tree;
    let account_data = [1u8; ARRAY_LEN];

    // V1 trees
    let (address, _) = light_sdk::address::v1::derive_address(
        &[b"compressed".as_slice(), account_data.as_slice()],
        &address_tree,
        &sdk_native_test::ID,
    );

    let v1_output_tree = rpc.test_accounts.v1_state_trees[0].merkle_tree;
    create_pda(
        &payer,
        &mut rpc,
        &v1_output_tree,
        account_data,
        address_tree,
        address,
    )
    .await
    .unwrap();

    let compressed_pda = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .clone()
        .unwrap();
    assert_eq!(compressed_pda.address.unwrap(), address);

    update_pda(&payer, &mut rpc, [2u8; ARRAY_LEN], compressed_pda.into())
        .await
        .unwrap();
}

pub async fn create_pda(
    payer: &Keypair,
    rpc: &mut LightProgramTest,
    merkle_tree_pubkey: &Pubkey,
    account_data: [u8; ARRAY_LEN],
    address_tree_pubkey: Pubkey,
    address: [u8; 32],
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_native_test::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts
        .add_system_accounts(system_account_meta_config)
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
        proof: rpc_result.proof.0.unwrap().into(),
        address_tree_info: packed_address_tree_info,
        data: account_data,
        output_merkle_tree_index,
        system_accounts_offset: system_accounts_offset as u8,
        tree_accounts_offset: tree_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: sdk_native_test::ID,
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
    new_account_data: [u8; ARRAY_LEN],
    compressed_account: CompressedAccountWithMerkleContext,
) -> Result<(), RpcError> {
    let system_account_meta_config = SystemAccountMetaConfig::new(sdk_native_test::ID);
    let mut accounts = PackedAccounts::default();
    accounts.add_pre_accounts_signer(payer.pubkey());
    accounts
        .add_system_accounts(system_account_meta_config)
        .unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash().unwrap()], vec![], None)
        .await?
        .value;

    let packed_accounts = rpc_result
        .pack_tree_infos(&mut accounts)
        .state_trees
        .unwrap();

    let meta = CompressedAccountMeta {
        tree_info: packed_accounts.packed_tree_infos[0],
        address: compressed_account.compressed_account.address.unwrap(),
        output_state_tree_index: packed_accounts.output_tree_index,
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
        proof: rpc_result.proof,
        new_data: new_account_data,
        system_accounts_offset: system_accounts_offset as u8,
    };
    let inputs = instruction_data.try_to_vec().unwrap();

    let instruction = Instruction {
        program_id: sdk_native_test::ID,
        accounts,
        data: [&[1u8][..], &inputs[..]].concat(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;
    Ok(())
}
