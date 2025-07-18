#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::merkle_tree::MerkleTreeExt,
};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts_v2, TestAccounts},
    program_test::LightProgramTest,
};
use light_prover_client::prover::{spawn_prover, ProverConfig, ProverMode};
use light_sdk::{
    account_meta::LightAccountMeta,
    address::derive_address,
    instruction_data::LightInstructionData,
    tree_info::{AddressTreeInfo, PackedAccounts},
    find_cpi_signer_macro,
    verify::find_cpi_signer,
    PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_NOOP,
};
use light_test_utils::{Rpc, RpcError};
use memo::MemoAccount;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_memo_program() {
     spawn_prover(ProverConfig::default()).await;

    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![("memo", memo::ID)])).await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer = TestIndexer::new(
        Vec::from(&[StateMerkleTreeAccounts {
            merkle_tree: env.v1_state_trees[0].merkle_tree,
            nullifier_queue: env.v1_state_trees[0].nullifier_queue,
            cpi_context: env.v1_state_trees[0].cpi_context,
        }]),
        Vec::from(&[AddressMerkleTreeAccounts {
            merkle_tree: env.v1_address_trees[0].merkle_tree,
            queue: env.v1_address_trees[0].queue,
        }]),
        payer.insecure_clone(),
        env.protocol.group_pda.clone(),
        None,
    )
    .await;

    let mut remaining_accounts = PackedAccounts::default();

    let address_merkle_context = AddressTreeInfo {
        address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: env.v1_address_trees[0].queue,
    };

    let (address, _) = derive_address(
        &[b"memo", payer.pubkey().as_ref()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &memo::ID,
    );

    let account_compression_authority = find_cpi_signer_macro!(&PROGRAM_ID_LIGHT_SYSTEM);
    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;

    // Create a memo
    let message = "Hello, world!".to_string();
    create_memo(
        &message,
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

    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&memo::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let memo = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let memo = MemoAccount::deserialize(&mut &memo[..]).unwrap();
    assert_eq!(memo.authority, payer.pubkey());
    assert_eq!(memo.message, "Hello, world!");

    let new_message = "Updated memo!".to_string();
    update_memo(
        &new_message,
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&memo::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let memo = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let memo = MemoAccount::deserialize(&mut &memo[..]).unwrap();
    assert_eq!(memo.message, "Updated memo!");

    delete_memo(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&memo::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 0);
}

#[allow(clippy::too_many_arguments)]
async fn create_memo<R>(
    message: &str,
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    env: &TestAccounts,
    remaining_accounts: &mut PackedAccounts,
    payer: &Keypair,
    address: &[u8; 32],
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: Rpc + MerkleTreeExt,
{
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.v1_address_trees[0].merkle_tree]),
            rpc,
        )
        .await
        .unwrap();

    let address_merkle_context = AddressTreeInfo {
        address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: env.v1_address_trees[0].queue,
    };
    let account = LightAccountMeta::new_init(
        &env.v1_state_trees[0].merkle_tree,
        Some(&address_merkle_context),
        Some(rpc_result.value.get_address_root_indices()[0]),
        remaining_accounts,
    )
    .unwrap();

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = memo::instruction::CreateMemo {
        inputs,
        message: message.to_string(),
    };

    let cpi_signer = find_cpi_signer(&memo::ID);

    let accounts = memo::accounts::CreateMemo {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: memo::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: memo::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_memo<R>(
    new_message: &str,
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    remaining_accounts: &mut PackedAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: Rpc + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(Vec::from(&[hash])),
            Some(Vec::from(&[merkle_tree_pubkey])),
            None,
            None,
            rpc,
        )
        .await
        .unwrap();

    let compressed_account = LightAccountMeta::new_mut(
        compressed_account,
        rpc_result.value.get_root_indices()[0].unwrap(),
        &merkle_tree_pubkey,
        remaining_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = memo::instruction::UpdateMemo {
        inputs,
        new_message: new_message.to_string(),
    };

    let cpi_signer = find_cpi_signer(&memo::ID);

    let accounts = memo::accounts::UpdateMemo {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: memo::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: memo::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn delete_memo<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    remaining_accounts: &mut PackedAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: Rpc + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(Vec::from(&[hash])),
            Some(Vec::from(&[merkle_tree_pubkey])),
            None,
            None,
            rpc,
        )
        .await
        .unwrap();

    let compressed_account = LightAccountMeta::new_close(
        compressed_account,
        rpc_result.value.get_root_indices()[0].unwrap(),
        remaining_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = memo::instruction::DeleteMemo { inputs };

    let cpi_signer = find_cpi_signer(&memo::ID);

    let accounts = memo::accounts::DeleteMemo {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: memo::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: memo::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}
