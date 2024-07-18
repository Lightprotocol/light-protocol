use anchor_spl::token::{Mint, TokenAccount};
use solana_program_test::BanksClientError;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};
use spl_token::instruction::initialize_mint;

use light_compressed_token::{
    burn::sdk::{create_burn_instruction, CreateBurnInstructionInputs},
    delegation::sdk::{
        create_approve_instruction, create_revoke_instruction, CreateApproveInstructionInputs,
        CreateRevokeInstructionInputs,
    },
    freeze::sdk::{create_instruction, CreateInstructionInputs},
    get_token_pool_pda,
    mint_sdk::{create_create_token_pool_instruction, create_mint_to_instruction},
    process_transfer::{
        get_cpi_authority_pda, transfer_sdk::create_transfer_instruction, TokenTransferOutputData,
    },
    token_data::AccountState,
    TokenData,
};
use light_hasher::Poseidon;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::MerkleContext, event::PublicTransactionEvent},
};

use crate::indexer::{Indexer, TestIndexer, TokenDataWithContext};
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;
use crate::{
    assert_compressed_tx::get_merkle_tree_snapshots,
    assert_token_tx::{assert_create_mint, assert_mint_to, assert_transfer},
    create_account_instruction,
    rpc::errors::RpcError,
};

pub async fn mint_tokens_helper<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    test_indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
) {
    let payer_pubkey = mint_authority.pubkey();
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        mint,
        merkle_tree_pubkey,
        amounts.clone(),
        recipients.clone(),
    );

    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&vec![*merkle_tree_pubkey; amounts.len()]);

    let snapshots = get_merkle_tree_snapshots::<R>(rpc, &output_merkle_tree_accounts).await;
    let previous_mint_supply =
        spl_token::state::Mint::unpack(&rpc.get_account(*mint).await.unwrap().unwrap().data)
            .unwrap()
            .supply;

    let pool: Pubkey = get_token_pool_pda(mint);
    let previous_pool_amount =
        spl_token::state::Account::unpack(&rpc.get_account(pool).await.unwrap().unwrap().data)
            .unwrap()
            .amount;
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &payer_pubkey,
            &[mint_authority],
            None,
        )
        .await
        .unwrap()
        .unwrap();

    let (_, created_token_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    assert_mint_to(
        rpc,
        test_indexer,
        &recipients,
        *mint,
        amounts.as_slice(),
        &snapshots,
        &created_token_accounts,
        previous_mint_supply,
        previous_pool_amount,
    )
    .await;
}

pub async fn create_token_pool<R: RpcConnection>(
    rpc: &mut R,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
    freeze_authority: Option<&Pubkey>,
    mint_keypair: Option<&Keypair>,
) -> Pubkey {
    let keypair = Keypair::new();
    let mint_keypair = match mint_keypair {
        Some(mint_keypair) => mint_keypair,
        None => &keypair,
    };
    let mint_pubkey = (*mint_keypair).pubkey();
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        mint_rent,
        &light_compressed_token::ID,
        Some(mint_keypair),
    );

    let create_mint_ix = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint_pubkey,
        mint_authority,
        freeze_authority,
        decimals,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[account_create_ix, create_mint_ix],
        &payer.pubkey(),
        &[payer],
    )
    .await
    .unwrap();
    mint_pubkey
}

pub async fn create_mint_helper<R: RpcConnection>(rpc: &mut R, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();
    let mint = Keypair::new();

    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    rpc.create_and_send_transaction(&instructions, &payer_pubkey, &[payer, &mint])
        .await
        .unwrap();
    assert_create_mint(rpc, &payer_pubkey, &mint.pubkey(), &pool).await;
    mint.pubkey()
}

pub async fn mint_wrapped_sol<R: RpcConnection>(
    rpc: &mut R,
    payer: &Keypair,
    token_account: &Pubkey,
    amount: u64,
) -> Result<Signature, RpcError> {
    let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &payer.pubkey(),
        token_account,
        amount,
    );
    let sync_native_ix = spl_token::instruction::sync_native(&spl_token::ID, token_account)
        .map_err(|e| RpcError::CustomError(format!("{:?}", e)))?;

    rpc.create_and_send_transaction(&[transfer_ix, sync_native_ix], &payer.pubkey(), &[payer])
        .await
}

pub fn create_initialize_mint_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
) -> ([Instruction; 4], Pubkey) {
    let account_create_ix =
        create_account_instruction(payer, Mint::LEN, rent, &spl_token::ID, Some(mint_keypair));

    let mint_pubkey = mint_keypair.pubkey();
    let create_mint_instruction = initialize_mint(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        authority,
        Some(authority),
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_create_token_pool_instruction(payer, &mint_pubkey);
    let pool_pubkey = get_token_pool_pda(&mint_pubkey);
    (
        [
            account_create_ix,
            create_mint_instruction,
            transfer_ix,
            instruction,
        ],
        pool_pubkey,
    )
}

/// Creates a spl token account and initializes it with the given mint and owner.
/// This function is useful to create token accounts for spl compression and decompression tests.
pub async fn create_token_account<R: RpcConnection>(
    rpc: &mut R,
    mint: &Pubkey,
    account_keypair: &Keypair,
    owner: &Keypair,
) -> Result<(), BanksClientError> {
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)
        .await
        .unwrap();
    let account_create_ix = create_account_instruction(
        &owner.pubkey(),
        TokenAccount::LEN,
        rent,
        &spl_token::ID,
        Some(account_keypair),
    );
    let instruction = spl_token::instruction::initialize_account(
        &spl_token::ID,
        &account_keypair.pubkey(),
        mint,
        &owner.pubkey(),
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[account_create_ix, instruction],
        &owner.pubkey(),
        &[account_keypair, owner],
    )
    .await
    .unwrap();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn compressed_transfer_test<R: RpcConnection, I: Indexer<R>>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    mint: &Pubkey,
    from: &Keypair,
    recipients: &[Pubkey],
    amounts: &[u64],
    input_compressed_accounts: &[TokenDataWithContext],
    output_merkle_tree_pubkeys: &[Pubkey],
    delegate_change_account_index: Option<u8>,
    transaction_params: Option<TransactionParams>,
) {
    if recipients.len() != amounts.len() && amounts.len() != output_merkle_tree_pubkeys.len() {
        panic!("recipients, amounts, and output_merkle_tree_pubkeys must have the same length");
    }
    let mut input_merkle_tree_context = Vec::new();
    let mut input_compressed_account_token_data = Vec::new();
    let mut input_compressed_account_hashes = Vec::new();
    let mut sum_input_amounts = 0;
    for account in input_compressed_accounts {
        let leaf_index = account.compressed_account.merkle_context.leaf_index;
        input_compressed_account_token_data.push(account.token_data);
        input_compressed_account_hashes.push(
            account
                .compressed_account
                .compressed_account
                .hash::<Poseidon>(
                    &account.compressed_account.merkle_context.merkle_tree_pubkey,
                    &leaf_index,
                )
                .unwrap(),
        );
        sum_input_amounts += account.token_data.amount;
        input_merkle_tree_context.push(MerkleContext {
            merkle_tree_pubkey: account.compressed_account.merkle_context.merkle_tree_pubkey,
            nullifier_queue_pubkey: account
                .compressed_account
                .merkle_context
                .nullifier_queue_pubkey,
            leaf_index,
            queue_index: None,
        });
    }

    let mut output_compressed_accounts = Vec::new();
    for ((recipient, amount), merkle_tree_pubkey) in recipients
        .iter()
        .zip(amounts)
        .zip(output_merkle_tree_pubkeys)
    {
        let account = TokenTransferOutputData {
            amount: *amount,
            owner: *recipient,
            lamports: None,
            merkle_tree: *merkle_tree_pubkey,
        };
        sum_input_amounts -= amount;
        output_compressed_accounts.push(account);
    }
    // add change compressed account if tokens are left
    if sum_input_amounts > 0 {
        let account = TokenTransferOutputData {
            amount: sum_input_amounts,
            owner: from.pubkey(),
            lamports: None,
            merkle_tree: *output_merkle_tree_pubkeys.last().unwrap(),
        };
        output_compressed_accounts.push(account);
    }
    let input_merkle_tree_pubkeys: Vec<Pubkey> = input_merkle_tree_context
        .iter()
        .map(|x| x.merkle_tree_pubkey)
        .collect();

    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    output_compressed_accounts.sort_by(|a, b| a.merkle_tree.cmp(&b.merkle_tree));

    let delegate_pubkey = if delegate_change_account_index.is_some() {
        Some(payer.pubkey())
    } else {
        None
    };
    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &from.pubkey(), // authority
        &input_merkle_tree_context,
        &output_compressed_accounts,
        &proof_rpc_result.root_indices,
        &Some(proof_rpc_result.proof),
        input_compressed_account_token_data.as_slice(), // input_token_data
        *mint,
        delegate_pubkey, // owner_if_delegate_change_account_index
        false,           // is_compress
        None,            // compression_amount
        None,            // token_pool_pda
        None,            // compress_or_decompress_token_account
        true,
        delegate_change_account_index,
    )
    .unwrap();
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let authority_signer = if delegate_change_account_index.is_some() {
        payer
    } else {
        from
    };
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &payer.pubkey(),
            &[payer, authority_signer],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();

    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    let delegates = if let Some(index) = delegate_change_account_index {
        let mut delegates = vec![None; created_output_accounts.len()];
        delegates[index as usize] = Some(payer.pubkey());
        Some(delegates)
    } else {
        None
    };
    assert_transfer(
        rpc,
        test_indexer,
        &output_compressed_accounts,
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        &input_compressed_account_hashes,
        &snapshots,
        &input_snapshots,
        &event,
        delegates,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn decompress_test<R: RpcConnection, I: Indexer<R>>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    recipient_token_account: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let max_amount: u64 = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum();
    let change_out_compressed_account = TokenTransferOutputData {
        amount: max_amount - amount,
        owner: payer.pubkey(),
        lamports: None,
        merkle_tree: *output_merkle_tree_pubkey,
    };
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    let mint = input_compressed_accounts[0].token_data.mint;
    let instruction = create_transfer_instruction(
        &rpc.get_payer().pubkey(),
        &payer.pubkey(), // authority
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect::<Vec<_>>(), // input_compressed_account_merkle_tree_pubkeys
        &[change_out_compressed_account], // output_compressed_accounts
        &proof_rpc_result.root_indices, // root_indices
        &Some(proof_rpc_result.proof),
        input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect::<Vec<_>>()
            .as_slice(), // input_token_data
        mint,                            // mint
        None,                            // owner_if_delegate_change_account_index
        false,                           // is_compress
        Some(amount),                    // compression_amount
        Some(get_token_pool_pda(&mint)), // token_pool_pda
        Some(*recipient_token_account),  // compress_or_decompress_token_account
        true,
        None,
    )
    .unwrap();
    let output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let recipient_token_account_data_pre = spl_token::state::Account::unpack(
        &rpc.get_account(*recipient_token_account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &context_payer.pubkey(),
            &[&context_payer, payer],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();

    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    assert_transfer(
        rpc,
        test_indexer,
        &[change_out_compressed_account],
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        None,
    )
    .await;

    let recipient_token_account_data = spl_token::state::Account::unpack(
        &rpc.get_account(*recipient_token_account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(
        recipient_token_account_data.amount,
        recipient_token_account_data_pre.amount + amount
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn compress_test<R: RpcConnection, I: Indexer<R>>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    amount: u64,
    mint: &Pubkey,
    output_merkle_tree_pubkey: &Pubkey,
    sender_token_account: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let output_compressed_account = TokenTransferOutputData {
        amount,
        owner: payer.pubkey(),
        lamports: None,
        merkle_tree: *output_merkle_tree_pubkey,
    };
    let approve_instruction = spl_token::instruction::approve(
        &spl_token::ID,
        sender_token_account,
        &get_cpi_authority_pda().0,
        &payer.pubkey(),
        &[&payer.pubkey()],
        amount,
    )
    .unwrap();

    let instruction = create_transfer_instruction(
        &rpc.get_payer().pubkey(),
        &payer.pubkey(),              // authority
        &Vec::new(),                  // input_compressed_account_merkle_tree_pubkeys
        &[output_compressed_account], // output_compressed_accounts
        &Vec::new(),                  // root_indices
        &None,
        &Vec::new(),                    // input_token_data
        *mint,                          // mint
        None,                           // owner_if_delegate_is_signer
        true,                           // is_compress
        Some(amount),                   // compression_amount
        Some(get_token_pool_pda(mint)), // token_pool_pda
        Some(*sender_token_account),    // compress_or_decompress_token_account
        true,
        None,
    )
    .unwrap();
    let output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_test_snapshots = Vec::new();
    let recipient_token_account_data_pre = spl_token::state::Account::unpack(
        &rpc.get_account(*sender_token_account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[approve_instruction, instruction],
            &payer.pubkey(),
            &[&context_payer, payer],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();

    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);

    assert_transfer(
        rpc,
        test_indexer,
        &[output_compressed_account],
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        Vec::new().as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        None,
    )
    .await;

    let recipient_token_account_data = spl_token::state::Account::unpack(
        &rpc.get_account(*sender_token_account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(
        recipient_token_account_data.amount,
        recipient_token_account_data_pre.amount - amount
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn approve_test<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    delegated_amount: u64,
    delegate: &Pubkey,
    delegated_compressed_account_merkle_tree: &Pubkey,
    change_compressed_account_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    let mint = input_compressed_accounts[0].token_data.mint;
    let inputs = CreateApproveInstructionInputs {
        fee_payer: rpc.get_payer().pubkey(),
        authority: authority.pubkey(),
        input_merkle_contexts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect(),
        mint,
        delegated_amount,
        delegated_compressed_account_merkle_tree: *delegated_compressed_account_merkle_tree,
        change_compressed_account_merkle_tree: *change_compressed_account_merkle_tree,
        delegate: *delegate,
        root_indices: proof_rpc_result.root_indices,
        proof: proof_rpc_result.proof,
    };

    let instruction = create_approve_instruction(inputs).unwrap();
    let output_merkle_tree_pubkeys = vec![
        *delegated_compressed_account_merkle_tree,
        *change_compressed_account_merkle_tree,
    ];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &authority.pubkey(),
            &[&context_payer, authority],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();
    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    let input_amount = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let change_amount = input_amount - delegated_amount;
    let expected_delegated_token_data = TokenData {
        mint,
        owner: authority.pubkey(),
        amount: delegated_amount,
        delegate: Some(*delegate),
        state: AccountState::Initialized,
    };
    let expected_change_token_data = TokenData {
        mint,
        owner: authority.pubkey(),
        amount: change_amount,
        delegate: None,
        state: AccountState::Initialized,
    };
    assert_eq!(
        expected_delegated_token_data,
        created_output_accounts[0].token_data
    );
    assert_eq!(
        expected_change_token_data,
        created_output_accounts[1].token_data
    );
    let expected_compressed_output_accounts = create_expected_token_output_data(
        vec![expected_delegated_token_data, expected_change_token_data],
        &output_merkle_tree_pubkeys,
    );
    // left in case e2e tests create a situation where the order of the output accounts is different by sorting
    // assert!(created_output_accounts
    //     .iter()
    //     .any(|y| y.token_data == expected_delegated_token_data));
    // assert!(created_output_accounts
    //     .iter()
    //     .any(|y| y.token_data == expected_change_token_data));

    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        Some(vec![Some(*delegate), None]),
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn revoke_test<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    output_account_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    let mint = input_compressed_accounts[0].token_data.mint;
    let inputs = CreateRevokeInstructionInputs {
        fee_payer: rpc.get_payer().pubkey(),
        authority: authority.pubkey(),
        input_merkle_contexts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect(),
        mint,
        output_account_merkle_tree: *output_account_merkle_tree,
        root_indices: proof_rpc_result.root_indices,
        proof: proof_rpc_result.proof,
    };

    let instruction = create_revoke_instruction(inputs).unwrap();
    let output_merkle_tree_pubkeys = vec![*output_account_merkle_tree];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &authority.pubkey(),
            &[&context_payer, authority],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();
    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    let input_amount = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let expected_token_data = TokenData {
        mint,
        owner: authority.pubkey(),
        amount: input_amount,
        delegate: None,
        state: AccountState::Initialized,
    };
    assert_eq!(expected_token_data, created_output_accounts[0].token_data);
    let expected_compressed_output_accounts =
        create_expected_token_output_data(vec![expected_token_data], &output_merkle_tree_pubkeys);

    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        None,
    )
    .await;
}

pub async fn freeze_test<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    freeze_or_thaw_test::<R, true>(
        authority,
        rpc,
        test_indexer,
        input_compressed_accounts,
        outputs_merkle_tree,
        transaction_params,
    )
    .await;
}

pub async fn thaw_test<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    freeze_or_thaw_test::<R, false>(
        authority,
        rpc,
        test_indexer,
        input_compressed_accounts,
        outputs_merkle_tree,
        transaction_params,
    )
    .await;
}

pub async fn freeze_or_thaw_test<R: RpcConnection, const FREEZE: bool>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    let mint = input_compressed_accounts[0].token_data.mint;
    let inputs = CreateInstructionInputs {
        fee_payer: rpc.get_payer().pubkey(),
        authority: authority.pubkey(),
        input_merkle_contexts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect(),
        outputs_merkle_tree: *outputs_merkle_tree,
        root_indices: proof_rpc_result.root_indices,
        proof: proof_rpc_result.proof,
    };

    let instruction = create_instruction::<FREEZE>(inputs).unwrap();
    let output_merkle_tree_pubkeys =
        vec![*outputs_merkle_tree; input_compressed_account_hashes.len()];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &authority.pubkey(),
            &[&context_payer, authority],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();
    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);

    let mut delegates = Vec::new();
    let mut expected_output_accounts = Vec::new();
    for account in input_compressed_accounts.iter() {
        let state = if FREEZE {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };
        let expected_token_data = TokenData {
            mint,
            owner: input_compressed_accounts[0].token_data.owner,
            amount: account.token_data.amount,
            delegate: account.token_data.delegate,
            state,
        };
        if let Some(delegate) = account.token_data.delegate {
            delegates.push(Some(delegate));
        } else {
            delegates.push(None);
        }
        expected_output_accounts.push(expected_token_data);
    }
    let expected_compressed_output_accounts =
        create_expected_token_output_data(expected_output_accounts, &output_merkle_tree_pubkeys);
    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        Some(delegates),
    )
    .await;
}
#[allow(clippy::too_many_arguments)]
pub async fn burn_test<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    change_account_merkle_tree: &Pubkey,
    burn_amount: u64,
    signer_is_delegate: bool,
    transaction_params: Option<TransactionParams>,
) {
    let (
        input_compressed_account_hashes,
        input_merkle_tree_pubkeys,
        mint,
        output_amount,
        instruction,
    ) = create_burn_test_instruction(
        authority,
        rpc,
        test_indexer,
        &input_compressed_accounts,
        change_account_merkle_tree,
        burn_amount,
        signer_is_delegate,
        BurnInstructionMode::Normal,
    )
    .await;
    let output_merkle_tree_pubkeys = vec![*change_account_merkle_tree; 1];
    let output_merkle_tree_test_snapshots = if output_amount > 0 {
        let output_merkle_tree_accounts =
            test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);

        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await
    } else {
        Vec::new()
    };

    let token_pool_pda_address = get_token_pool_pda(&mint);
    let pre_token_pool_account = rpc
        .get_account(token_pool_pda_address)
        .await
        .unwrap()
        .unwrap();
    let pre_token_pool_balance = spl_token::state::Account::unpack(&pre_token_pool_account.data)
        .unwrap()
        .amount;

    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &authority.pubkey(),
            &[&context_payer, authority],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();
    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
    let mut delegates = Vec::new();
    let mut expected_output_accounts = Vec::new();

    let delegate = if signer_is_delegate {
        Some(authority.pubkey())
    } else {
        None
    };
    if output_amount > 0 {
        let expected_token_data = TokenData {
            mint,
            owner: input_compressed_accounts[0].token_data.owner,
            amount: output_amount,
            delegate,
            state: AccountState::Initialized,
        };
        if let Some(delegate) = expected_token_data.delegate {
            delegates.push(Some(delegate));
        } else {
            delegates.push(None);
        }
        expected_output_accounts.push(expected_token_data);
    }
    let expected_compressed_output_accounts =
        create_expected_token_output_data(expected_output_accounts, &output_merkle_tree_pubkeys);
    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        Some(delegates),
    )
    .await;
    let post_token_pool_account = rpc
        .get_account(token_pool_pda_address)
        .await
        .unwrap()
        .unwrap();
    let post_token_pool_balance = spl_token::state::Account::unpack(&post_token_pool_account.data)
        .unwrap()
        .amount;
    assert_eq!(
        post_token_pool_balance,
        pre_token_pool_balance - burn_amount
    );
}

#[derive(Debug, Clone, PartialEq)]
pub enum BurnInstructionMode {
    Normal,
    InvalidProof,
    InvalidMint,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_burn_test_instruction<R: RpcConnection>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: &[TokenDataWithContext],
    change_account_merkle_tree: &Pubkey,
    burn_amount: u64,
    signer_is_delegate: bool,
    mode: BurnInstructionMode,
) -> (Vec<[u8; 32]>, Vec<Pubkey>, Pubkey, u64, Instruction) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            rpc,
        )
        .await;
    let mint = if mode == BurnInstructionMode::InvalidMint {
        Pubkey::new_unique()
    } else {
        input_compressed_accounts[0].token_data.mint
    };
    let proof = if mode == BurnInstructionMode::InvalidProof {
        CompressedProof {
            a: proof_rpc_result.proof.a,
            b: proof_rpc_result.proof.b,
            c: proof_rpc_result.proof.a, // flip c to make proof invalid but not run into decompress errors
        }
    } else {
        proof_rpc_result.proof
    };
    let inputs = CreateBurnInstructionInputs {
        fee_payer: rpc.get_payer().pubkey(),
        authority: authority.pubkey(),
        input_merkle_contexts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect(),
        change_account_merkle_tree: *change_account_merkle_tree,
        root_indices: proof_rpc_result.root_indices,
        proof,
        mint,
        signer_is_delegate,
        burn_amount,
    };
    let input_amount_sum = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let output_amount = input_amount_sum - burn_amount;
    let instruction = create_burn_instruction(inputs).unwrap();
    (
        input_compressed_account_hashes,
        input_merkle_tree_pubkeys,
        mint,
        output_amount,
        instruction,
    )
}

pub fn create_expected_token_output_data(
    expected_token_data: Vec<TokenData>,
    merkle_tree_pubkeys: &[Pubkey],
) -> Vec<TokenTransferOutputData> {
    let mut expected_compressed_output_accounts = Vec::new();
    for (token_data, merkle_tree_pubkey) in
        expected_token_data.iter().zip(merkle_tree_pubkeys.iter())
    {
        expected_compressed_output_accounts.push(TokenTransferOutputData {
            owner: token_data.owner,
            amount: token_data.amount,
            merkle_tree: *merkle_tree_pubkey,
            lamports: None,
        });
    }
    expected_compressed_output_accounts
}
