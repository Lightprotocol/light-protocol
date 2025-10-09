#![cfg(feature = "test-sbf")]

use anchor_lang::AnchorDeserialize;
use light_client::indexer::CompressedAccount;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig,
};
use light_sdk::{
    address::v1::derive_address,
    instruction::{
        account_meta::CompressedAccountMetaBurn, PackedAccounts, SystemAccountMetaConfig,
    },
};
use light_test_utils::{Rpc, RpcError};
use sdk_anchor_test::MyCompressedAccount;
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signature, Signer},
};

#[serial]
#[tokio::test]
async fn test_read_sha256() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"compressed", b"readonly_sha_test".as_slice()],
        &address_tree_info.tree,
        &sdk_anchor_test::ID,
    );

    // Create a compressed account to test read-only on
    create_compressed_account("readonly_sha_test".to_string(), &mut rpc, &payer, &address)
        .await
        .unwrap();

    // Get the created account
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Test read_sha256_light_system_cpi
    read_sha256_light_system_cpi(&mut rpc, &payer, compressed_account.clone())
        .await
        .unwrap();

    // Test read_sha256_lowlevel
    read_sha256_lowlevel(&mut rpc, &payer, compressed_account)
        .await
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_read_poseidon() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"compressed", b"readonly_poseidon_test".as_slice()],
        &address_tree_info.tree,
        &sdk_anchor_test::ID,
    );

    // Create a compressed account with Poseidon hashing to test read-only on
    create_compressed_account_poseidon(
        "readonly_poseidon_test".to_string(),
        &mut rpc,
        &payer,
        &address,
    )
    .await
    .unwrap();

    // Get the created account
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Test read_poseidon_light_system_cpi
    read_poseidon_light_system_cpi(&mut rpc, &payer, compressed_account.clone())
        .await
        .unwrap();

    // Test read_poseidon_lowlevel
    read_poseidon_lowlevel(&mut rpc, &payer, compressed_account)
        .await
        .unwrap();
}

async fn create_compressed_account(
    name: String,
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<Signature, RpcError> {
    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_system_accounts(config).unwrap();

    let address_merkle_tree_info = rpc.get_address_tree_v1();

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: *address,
                tree: address_merkle_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;
    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::CreateCompressedAccount {
                proof: rpc_result.proof,
                address_tree_info: packed_accounts.address_trees[0],
                output_tree_index,
                name,
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn read_sha256_light_system_cpi(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::ReadSha256LightSystemCpi {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMetaBurn {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn read_sha256_lowlevel(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::ReadSha256Lowlevel {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMetaBurn {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn create_compressed_account_poseidon(
    name: String,
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<Signature, RpcError> {
    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_system_accounts(config).unwrap();

    let address_merkle_tree_info = rpc.get_address_tree_v1();

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: *address,
                tree: address_merkle_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;
    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::CreateCompressedAccountPoseidon {
                proof: rpc_result.proof,
                address_tree_info: packed_accounts.address_trees[0],
                output_tree_index,
                name,
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn read_poseidon_light_system_cpi(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::ReadPoseidonLightSystemCpi {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMetaBurn {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn read_poseidon_lowlevel(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::ReadPoseidonLowlevel {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMetaBurn {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
