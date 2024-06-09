use crate::{
    assert_compressed_tx::get_merkle_tree_snapshots,
    assert_token_tx::{assert_create_mint, assert_mint_to, assert_transfer},
    create_account_instruction,
};

use crate::indexer::{TestIndexer, TokenDataWithContext};
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;
use light_compressed_token::{
    get_cpi_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initialize_mint_instruction, create_mint_to_instruction},
    transfer_sdk::create_transfer_instruction,
    TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_system_program::sdk::{compressed_account::MerkleContext, event::PublicTransactionEvent};
use solana_program_test::BanksClientError;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::instruction::initialize_mint;
use spl_token::state::Mint;

pub async fn mint_tokens_helper<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<INDEXED_ARRAY_SIZE, R>,
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

    let snapshots =
        get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(rpc, &output_merkle_tree_accounts).await;
    let previous_mint_supply =
        spl_token::state::Mint::unpack(&rpc.get_account(*mint).await.unwrap().unwrap().data)
            .unwrap()
            .supply;

    let pool: Pubkey = get_token_pool_pda(mint);
    let previous_pool_amount =
        spl_token::state::Account::unpack(&rpc.get_account(pool).await.unwrap().unwrap().data)
            .unwrap()
            .amount;
    let (event, _signature) = rpc
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

pub async fn create_mint<R: RpcConnection>(
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
        .get_minimum_balance_for_rent_exemption(anchor_spl::token::Mint::LEN)
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
pub fn create_initialize_mint_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
) -> ([Instruction; 4], Pubkey) {
    let account_create_ix = create_account_instruction(
        payer,
        anchor_spl::token::Mint::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let create_mint_instruction = initialize_mint(
        &anchor_spl::token::ID,
        &mint_keypair.pubkey(),
        authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_initialize_mint_instruction(payer, &mint_pubkey);
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
        .get_minimum_balance_for_rent_exemption(anchor_spl::token::TokenAccount::LEN)
        .await
        .unwrap();
    let account_create_ix = create_account_instruction(
        &owner.pubkey(),
        anchor_spl::token::TokenAccount::LEN,
        rent,
        &anchor_spl::token::ID,
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
pub async fn compressed_transfer_test<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<INDEXED_ARRAY_SIZE, R>,
    mint: &Pubkey,
    from: &Keypair,
    recipients: &[Pubkey],
    amounts: &[u64],
    input_compressed_accounts: &[TokenDataWithContext],
    output_merkle_tree_pubkeys: &[Pubkey],
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

    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &from.pubkey(), // authority
        &input_merkle_tree_context,
        &output_compressed_accounts,
        &proof_rpc_result.root_indices,
        &Some(proof_rpc_result.proof),
        input_compressed_account_token_data.as_slice(), // input_token_data
        *mint,
        None,  // owner_if_delegate_is_signer
        false, // is_compress
        None,  // compression_amount
        None,  // token_pool_pda
        None,  // compress_or_decompress_token_account
        true,
    )
    .unwrap();
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let snapshots = get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(
        rpc,
        output_merkle_tree_accounts.as_slice(),
    )
    .await;
    let input_snapshots = get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(
        rpc,
        input_merkle_tree_accounts.as_slice(),
    )
    .await;
    let (event, _signature) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &payer.pubkey(),
            &[payer, from],
            transaction_params,
        )
        .await
        .unwrap()
        .unwrap();

    let (_, created_output_accounts) = test_indexer.add_event_and_compressed_accounts(&event);
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
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn decompress_test<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<INDEXED_ARRAY_SIZE, R>,
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
        None,                            // owner_if_delegate_is_signer
        false,                           // is_compress
        Some(amount),                    // compression_amount
        Some(get_token_pool_pda(&mint)), // token_pool_pda
        Some(*recipient_token_account),  // compress_or_decompress_token_account
        true,
    )
    .unwrap();
    let output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots = get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(
        rpc,
        output_merkle_tree_accounts.as_slice(),
    )
    .await;
    let input_merkle_tree_test_snapshots = get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(
        rpc,
        input_merkle_tree_accounts.as_slice(),
    )
    .await;
    let recipient_token_account_data_pre = spl_token::state::Account::unpack(
        &rpc.get_account(*recipient_token_account)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature) = rpc
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
pub async fn compress_test<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<INDEXED_ARRAY_SIZE, R>,
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
        &anchor_spl::token::ID,
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
    )
    .unwrap();
    let output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);
    let output_merkle_tree_test_snapshots = get_merkle_tree_snapshots::<INDEXED_ARRAY_SIZE, R>(
        rpc,
        output_merkle_tree_accounts.as_slice(),
    )
    .await;
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
    let (event, _signature) = rpc
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
