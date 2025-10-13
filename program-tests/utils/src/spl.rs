use anchor_spl::token::{Mint, TokenAccount};
use forester_utils::instructions::create_account::create_account_instruction;
use light_client::{
    fee::TransactionParams,
    indexer::Indexer,
    rpc::{errors::RpcError, Rpc},
};
use light_compressed_account::{
    compressed_account::MerkleContext, instruction_data::compressed_proof::CompressedProof,
    TreeType,
};
use light_compressed_token::{
    burn::sdk::{create_burn_instruction, CreateBurnInstructionInputs},
    constants::NUM_MAX_POOL_ACCOUNTS,
    delegation::sdk::{
        create_approve_instruction, create_revoke_instruction, CreateApproveInstructionInputs,
        CreateRevokeInstructionInputs,
    },
    freeze::sdk::{create_instruction, CreateInstructionInputs},
    get_token_pool_pda, get_token_pool_pda_with_index,
    mint_sdk::{
        create_add_token_pool_instruction, create_create_token_pool_instruction,
        create_mint_to_instruction,
    },
    process_compress_spl_token_account::sdk::create_compress_spl_token_account_instruction,
    process_transfer::{transfer_sdk::create_transfer_instruction, TokenTransferOutputData},
};
use light_ctoken_types::state::{CompressedTokenAccountState, TokenData};
use light_hasher::Poseidon;
use light_program_test::{indexer::TestIndexerExtensions, program_test::TestRpc};
use light_sdk::token::TokenDataWithMerkleContext;
use solana_banks_client::BanksClientError;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};
use spl_token::instruction::initialize_mint;

use crate::{
    assert_compressed_tx::get_merkle_tree_snapshots,
    assert_token_tx::{assert_create_mint, assert_mint_to, assert_transfer},
    conversions::{program_to_sdk_token_data, sdk_to_program_token_data},
};

pub async fn mint_tokens_helper<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
) {
    mint_tokens_helper_with_lamports(
        rpc,
        test_indexer,
        merkle_tree_pubkey,
        mint_authority,
        mint,
        amounts,
        recipients,
        None,
    )
    .await
}

pub async fn mint_spl_tokens<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    token_account: &Pubkey,
    token_owner: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
    is_token_22: bool,
) -> Result<Signature, RpcError> {
    let mint_to_instruction = if is_token_22 {
        spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            mint,
            token_account,
            token_owner,
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap()
    } else {
        spl_token::instruction::mint_to(
            &spl_token::ID,
            mint,
            token_account,
            token_owner,
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap()
    };
    rpc.create_and_send_transaction(
        &[mint_to_instruction],
        &mint_authority.pubkey(),
        &[mint_authority],
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn mint_tokens_helper_with_lamports<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
    lamports: Option<u64>,
) {
    mint_tokens_22_helper_with_lamports(
        rpc,
        test_indexer,
        merkle_tree_pubkey,
        mint_authority,
        mint,
        amounts,
        recipients,
        lamports,
        false,
    )
    .await;
}
#[allow(clippy::too_many_arguments)]
pub async fn mint_tokens_22_helper_with_lamports<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
    lamports: Option<u64>,
    token_22: bool,
) {
    mint_tokens_22_helper_with_lamports_and_bump(
        rpc,
        test_indexer,
        merkle_tree_pubkey,
        mint_authority,
        mint,
        amounts,
        recipients,
        lamports,
        token_22,
        0,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn mint_tokens_22_helper_with_lamports_and_bump<
    R: Rpc,
    I: Indexer + TestIndexerExtensions,
>(
    rpc: &mut R,
    test_indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
    lamports: Option<u64>,
    token_22: bool,
    token_pool_index: u8,
) {
    let payer_pubkey = mint_authority.pubkey();
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        mint,
        merkle_tree_pubkey,
        amounts.clone(),
        recipients.clone(),
        lamports,
        token_22,
        token_pool_index,
    );

    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&vec![*merkle_tree_pubkey; amounts.len()]);

    let snapshots = get_merkle_tree_snapshots::<R>(rpc, &output_merkle_tree_accounts).await;
    let previous_mint_supply =
        spl_token::state::Mint::unpack(&rpc.get_account(*mint).await.unwrap().unwrap().data)
            .unwrap()
            .supply;

    let pool: Pubkey = get_token_pool_pda_with_index(mint, token_pool_index);
    let previous_pool_amount =
        spl_token::state::Account::unpack(&rpc.get_account(pool).await.unwrap().unwrap().data)
            .unwrap()
            .amount;
    let (event, _signature, _) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer_pubkey,
            &[mint_authority],
        )
        .await
        .unwrap()
        .unwrap();
    let slot = rpc.get_slot().await.unwrap();

    let (_, created_token_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());

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
        pool,
    )
    .await;
}

pub async fn create_token_pool<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
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
    let (instructions, _) = create_initialize_mint_instructions(
        &payer.pubkey(),
        mint_authority,
        mint_rent,
        decimals,
        mint_keypair,
    );
    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer, mint_keypair])
        .await
        .unwrap();
    mint_pubkey
}

pub async fn create_mint_helper<R: Rpc>(rpc: &mut R, payer: &Keypair) -> Pubkey {
    let mint = Keypair::new();
    create_mint_helper_with_keypair(rpc, payer, &mint).await
}

pub async fn create_mint_helper_with_keypair<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Keypair,
) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, mint);

    let _ = rpc
        .create_and_send_transaction(&instructions, &payer_pubkey, &[payer, mint])
        .await
        .unwrap();
    assert_create_mint(rpc, &payer_pubkey, &mint.pubkey(), &pool).await;
    mint.pubkey()
}

pub async fn create_mint_22_helper<R: Rpc>(rpc: &mut R, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();
    let mint = Keypair::new();

    let (instructions, pool) =
        create_initialize_mint_22_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint, true);

    rpc.create_and_send_transaction(&instructions, &payer_pubkey, &[payer, &mint])
        .await
        .unwrap();
    assert_create_mint(rpc, &payer_pubkey, &mint.pubkey(), &pool).await;
    mint.pubkey()
}

pub async fn mint_wrapped_sol<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    token_account: &Pubkey,
    amount: u64,
    is_token_22: bool,
) -> Result<Signature, RpcError> {
    let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &payer.pubkey(),
        token_account,
        amount,
    );
    let sync_native_ix = if is_token_22 {
        spl_token_2022::instruction::sync_native(&spl_token_2022::ID, token_account)
            .map_err(|e| RpcError::CustomError(format!("{:?}", e)))?
    } else {
        spl_token::instruction::sync_native(&spl_token::ID, token_account)
            .map_err(|e| RpcError::CustomError(format!("{:?}", e)))?
    };

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
    create_initialize_mint_22_instructions(payer, authority, rent, decimals, mint_keypair, false)
}

pub fn create_initialize_mint_22_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
    token_22: bool,
) -> ([Instruction; 4], Pubkey) {
    let program_id = if token_22 {
        anchor_spl::token_2022::ID
    } else {
        spl_token::ID
    };
    let account_create_ix =
        create_account_instruction(payer, Mint::LEN, rent, &program_id, Some(mint_keypair));

    let mint_pubkey = mint_keypair.pubkey();
    let create_mint_instruction = if token_22 {
        spl_token_2022::instruction::initialize_mint(
            &program_id,
            &mint_keypair.pubkey(),
            authority,
            Some(authority),
            decimals,
        )
        .unwrap()
    } else {
        initialize_mint(
            &program_id,
            &mint_keypair.pubkey(),
            authority,
            Some(authority),
            decimals,
        )
        .unwrap()
    };
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_create_token_pool_instruction(payer, &mint_pubkey, token_22);

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

pub async fn create_additional_token_pools<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    is_token_22: bool,
    num: u8,
) -> Result<Vec<Pubkey>, RpcError> {
    let mut instructions = Vec::new();
    let mut created_token_pools = Vec::new();

    for token_pool_index in 0..NUM_MAX_POOL_ACCOUNTS {
        if instructions.len() == num as usize {
            break;
        }
        let token_pool_pda = get_token_pool_pda_with_index(mint, token_pool_index);
        let account = rpc.get_account(token_pool_pda).await.unwrap();
        println!("bump {}", token_pool_index);
        println!("account exists {:?}", account.is_some());
        if account.is_none() {
            created_token_pools.push(token_pool_pda);
            let instruction = create_add_token_pool_instruction(
                &payer.pubkey(),
                mint,
                token_pool_index,
                is_token_22,
            );
            instructions.push(instruction);
        }
    }
    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await?;
    Ok(created_token_pools)
}

/// Creates a spl token account and initializes it with the given mint and owner.
/// This function is useful to create token accounts for spl compression and decompression tests.
pub async fn create_token_account<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    account_keypair: &Keypair,
    owner: &Keypair,
) -> Result<(), BanksClientError> {
    create_token_2022_account(rpc, mint, account_keypair, owner, false).await
}
pub async fn create_token_2022_account<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
    account_keypair: &Keypair,
    owner: &Keypair,
    token_22: bool,
) -> Result<(), BanksClientError> {
    let account_len = if token_22 {
        spl_token_2022::state::Account::LEN
    } else {
        spl_token::state::Account::LEN
    };
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(account_len)
        .await
        .unwrap();
    let program_id = if token_22 {
        spl_token_2022::ID
    } else {
        spl_token::ID
    };
    let account_create_ix = create_account_instruction(
        &owner.pubkey(),
        TokenAccount::LEN,
        rent,
        &program_id,
        Some(account_keypair),
    );
    let instruction = if token_22 {
        spl_token_2022::instruction::initialize_account(
            &program_id,
            &account_keypair.pubkey(),
            mint,
            &owner.pubkey(),
        )
        .unwrap()
    } else {
        spl_token::instruction::initialize_account(
            &program_id,
            &account_keypair.pubkey(),
            mint,
            &owner.pubkey(),
        )
        .unwrap()
    };
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
pub async fn compressed_transfer_test<
    R: Rpc + light_program_test::program_test::TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    mint: &Pubkey,
    from: &Keypair,
    recipients: &[Pubkey],
    amounts: &[u64],
    lamports: Option<Vec<Option<u64>>>,
    input_compressed_accounts: &[TokenDataWithMerkleContext],
    output_merkle_tree_pubkeys: &[Pubkey],
    delegate_change_account_index: Option<u8>,
    delegate_is_signer: bool,
    transaction_params: Option<TransactionParams>,
) {
    compressed_transfer_22_test(
        payer,
        rpc,
        test_indexer,
        mint,
        from,
        recipients,
        amounts,
        lamports,
        input_compressed_accounts,
        output_merkle_tree_pubkeys,
        delegate_change_account_index,
        delegate_is_signer,
        transaction_params,
        false,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn compressed_transfer_22_test<
    R: Rpc + light_program_test::program_test::TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    mint: &Pubkey,
    from: &Keypair,
    recipients: &[Pubkey],
    amounts: &[u64],
    mut lamports: Option<Vec<Option<u64>>>,
    input_compressed_accounts: &[TokenDataWithMerkleContext],
    output_merkle_tree_pubkeys: &[Pubkey],
    delegate_change_account_index: Option<u8>,
    delegate_is_signer: bool,
    transaction_params: Option<TransactionParams>,
    token_22: bool,
) {
    if recipients.len() != amounts.len() && amounts.len() != output_merkle_tree_pubkeys.len() {
        println!("{:?}", recipients);
        println!("{:?}", amounts);
        println!("{:?}", output_merkle_tree_pubkeys);
        panic!("recipients, amounts, and output_merkle_tree_pubkeys must have the same length");
    }
    let mut input_merkle_tree_context = Vec::new();
    let mut input_compressed_account_token_data = Vec::new();
    let mut input_compressed_account_hashes = Vec::new();
    let mut sum_input_amounts = 0;
    for account in input_compressed_accounts {
        let leaf_index = account.compressed_account.merkle_context.leaf_index;
        input_compressed_account_token_data.push(account.token_data.clone());
        input_compressed_account_hashes.push(account.compressed_account.hash().unwrap());
        sum_input_amounts += account.token_data.amount;
        input_merkle_tree_context.push(account.compressed_account.merkle_context);
    }
    let output_lamports = lamports
        .clone()
        .unwrap_or_else(|| vec![None; recipients.len()]);
    let mut output_compressed_accounts = Vec::new();
    for (((recipient, amount), merkle_tree_pubkey), lamports) in recipients
        .iter()
        .zip(amounts)
        .zip(output_merkle_tree_pubkeys)
        .zip(output_lamports)
    {
        let account = TokenTransferOutputData {
            amount: *amount,
            owner: *recipient,
            lamports,
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
        .map(|x| x.merkle_tree_pubkey.into())
        .collect();
    println!("{:?}", input_compressed_accounts);
    println!(
        "input_compressed_account_hashes: {:?}",
        input_compressed_account_hashes
    );
    let rpc_result = test_indexer
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
    output_compressed_accounts.sort_by(|a, b| a.merkle_tree.cmp(&b.merkle_tree));

    let delegate_pubkey = if delegate_is_signer {
        Some(payer.pubkey())
    } else {
        None
    };

    let authority_signer = if delegate_is_signer { payer } else { from };

    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &authority_signer.pubkey(), // authority
        &input_merkle_tree_context,
        &output_compressed_accounts,
        &rpc_result.value.get_root_indices(),
        &rpc_result.value.proof.0,
        input_compressed_account_token_data
            .iter()
            .cloned()
            .map(sdk_to_program_token_data)
            .collect::<Vec<_>>()
            .as_slice(), // input_token_data
        input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account.compressed_account)
            .cloned()
            .collect::<Vec<_>>()
            .as_slice(),
        *mint,
        delegate_pubkey, // owner_if_delegate_change_account_index
        false,           // is_compress
        None,            // compression_amount
        None,            // token_pool_pda
        None,            // compress_or_decompress_token_account
        true,
        delegate_change_account_index,
        None,
        token_22,
        &[],
        false,
    )
    .unwrap();
    let sum_input_lamports = input_compressed_accounts
        .iter()
        .map(|x| &x.compressed_account.compressed_account.lamports)
        .sum::<u64>();
    let sum_output_lamports = output_compressed_accounts
        .iter()
        .map(|x| x.lamports.unwrap_or(0))
        .sum::<u64>();
    let sum_output_amounts = output_compressed_accounts
        .iter()
        .map(|x| x.amount)
        .sum::<u64>();
    let output_merkle_tree_pubkeys = if sum_input_lamports > sum_output_lamports
        || sum_input_amounts > sum_output_amounts && delegate_is_signer
    {
        let mut output_merkle_tree_pubkeys = output_merkle_tree_pubkeys.to_vec();
        output_merkle_tree_pubkeys.push(*output_merkle_tree_pubkeys.last().unwrap());
        if let Some(lamports) = &mut lamports {
            if sum_input_lamports != sum_output_lamports {
                lamports.push(Some(sum_input_lamports - sum_output_lamports));
            } else {
                lamports.push(None);
            }
        }
        output_merkle_tree_pubkeys
    } else {
        output_merkle_tree_pubkeys.to_vec()
    };

    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(output_merkle_tree_pubkeys.as_slice());
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&input_merkle_tree_pubkeys);
    let snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;

    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &payer.pubkey(),
        &[payer, authority_signer],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (created_change_output_account, created_token_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());
    let delegates = if let Some(index) = delegate_change_account_index {
        let mut delegates = vec![None; created_token_output_accounts.len()];
        delegates[index as usize] = Some(payer.pubkey());
        Some(delegates)
    } else {
        None
    };
    let mut created_output_accounts = Vec::new();
    created_token_output_accounts.iter().for_each(|x| {
        created_output_accounts.push(x.compressed_account.clone());
    });
    created_change_output_account.iter().for_each(|x| {
        created_output_accounts.push(x.clone());
    });
    assert_transfer(
        rpc,
        test_indexer,
        &output_compressed_accounts,
        created_output_accounts
            .into_iter()
            .collect::<Vec<_>>()
            .as_slice(),
        lamports,
        &input_compressed_account_hashes,
        &snapshots,
        &input_snapshots,
        &event,
        delegates,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn decompress_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    recipient_token_account: &Pubkey,
    transaction_params: Option<TransactionParams>,
    is_token_22: bool,
    token_pool_index: u8,
    additional_pool_accounts: Option<Vec<Pubkey>>,
) {
    let max_amount: u64 = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum();
    println!("max_amount: {}", max_amount);
    println!("amount: {}", amount);
    let output_amount = max_amount - amount;
    let change_out_compressed_account = TokenTransferOutputData {
        amount: output_amount,
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
        .map(|x| {
            x.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .into()
        })
        .collect::<Vec<_>>();
    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
    let mint = input_compressed_accounts[0].token_data.mint;
    let token_pool_pda = get_token_pool_pda_with_index(&mint, token_pool_index);

    let instruction = create_transfer_instruction(
        &rpc.get_payer().pubkey(),
        &payer.pubkey(), // authority
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect::<Vec<_>>(), // input_compressed_account_merkle_tree_pubkeys
        &[change_out_compressed_account], // output_compressed_accounts
        &proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Some(proof_rpc_result.value.proof.0.unwrap_or_default()),
        input_compressed_accounts
            .iter()
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect::<Vec<_>>()
            .as_slice(), // input_token_data
        &input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account.compressed_account)
            .cloned()
            .collect::<Vec<_>>(),
        mint,                           // mint
        None,                           // owner_if_delegate_change_account_index
        false,                          // is_compress
        Some(amount),                   // compression_amount
        Some(token_pool_pda),           // token_pool_pda
        Some(*recipient_token_account), // compress_or_decompress_token_account
        true,
        None,
        None,
        is_token_22,
        additional_pool_accounts
            .clone()
            .unwrap_or_default()
            .as_slice(),
        false,
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
    let mut token_pool_pre_balances = vec![
        spl_token::state::Account::unpack(
            &rpc.get_account(token_pool_pda).await.unwrap().unwrap().data,
        )
        .unwrap()
        .amount,
    ];
    for additional_pool_account in additional_pool_accounts
        .clone()
        .unwrap_or_default()
        .as_slice()
    {
        token_pool_pre_balances.push(
            spl_token::state::Account::unpack(
                &rpc.get_account(*additional_pool_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .data,
            )
            .unwrap()
            .amount,
        );
    }
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &context_payer.pubkey(),
        &[&context_payer, payer],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());
    assert_transfer(
        rpc,
        test_indexer,
        &[change_out_compressed_account],
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        None,
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
    println!("amount: {}", amount);
    println!("token_pool_pre_balances[0] {}", token_pool_pre_balances[0]);
    assert_eq!(
        recipient_token_account_data.amount,
        recipient_token_account_data_pre.amount + amount
    );
    let token_pool_post_balance = spl_token::state::Account::unpack(
        &rpc.get_account(token_pool_pda).await.unwrap().unwrap().data,
    )
    .unwrap()
    .amount;
    assert_eq!(
        token_pool_post_balance,
        token_pool_pre_balances[0].saturating_sub(amount)
    );
    let mut amount = amount.saturating_sub(token_pool_pre_balances[0]);
    for (i, additional_account) in additional_pool_accounts
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let post_balance = spl_token::state::Account::unpack(
            &rpc.get_account(*additional_account)
                .await
                .unwrap()
                .unwrap()
                .data,
        )
        .unwrap()
        .amount;
        amount = amount.saturating_sub(token_pool_pre_balances[i + 1]);
        if amount == 0 {
            break;
        }
        assert_eq!(
            post_balance,
            token_pool_pre_balances[i + 1].saturating_sub(amount)
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_compress_spl_token_account<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    payer: &Keypair,
    token_owner: &Keypair,
    mint: &Pubkey,
    token_account: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    remaining_amount: Option<u64>,
    is_token_22: bool,
    token_pool_index: u8,
) -> Result<(), RpcError> {
    let pre_token_account_amount = spl_token::state::Account::unpack(
        &rpc.get_account(*token_account).await.unwrap().unwrap().data,
    )
    .unwrap()
    .amount;
    let instruction = create_compress_spl_token_account_instruction(
        &token_owner.pubkey(),
        remaining_amount,
        None,
        &payer.pubkey(),
        &token_owner.pubkey(),
        mint,
        merkle_tree_pubkey,
        token_account,
        is_token_22,
        token_pool_index,
    );
    let (event, _, slot) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &token_owner.pubkey(),
            &[payer, token_owner],
        )
        .await?
        .unwrap();
    test_indexer.add_event_and_compressed_accounts(slot, &event.clone());

    let created_compressed_token_account = test_indexer
        .get_compressed_token_accounts_by_owner(&token_owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items[0]
        .clone();
    let expected_token_data = TokenData {
        amount: pre_token_account_amount - remaining_amount.unwrap_or_default(),
        mint: (*mint).into(),
        owner: token_owner.pubkey().into(),
        state: CompressedTokenAccountState::Initialized as u8,
        delegate: None,
        tlv: None,
    };
    assert_eq!(
        created_compressed_token_account.token,
        program_to_sdk_token_data(expected_token_data)
    );
    assert_eq!(
        created_compressed_token_account.account.tree_info.tree,
        *merkle_tree_pubkey
    );
    if let Some(remaining_amount) = remaining_amount {
        let post_token_account_amount = spl_token::state::Account::unpack(
            &rpc.get_account(*token_account).await.unwrap().unwrap().data,
        )
        .unwrap()
        .amount;
        assert_eq!(post_token_account_amount, remaining_amount);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn compress_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    amount: u64,
    mint: &Pubkey,
    output_merkle_tree_pubkey: &Pubkey,
    sender_token_account: &Pubkey,
    transaction_params: Option<TransactionParams>,
    is_token_22: bool,
    token_pool_index: u8,
    additional_pool_accounts: Option<Vec<Pubkey>>,
) {
    let output_compressed_account = TokenTransferOutputData {
        amount,
        owner: payer.pubkey(),
        lamports: None,
        merkle_tree: *output_merkle_tree_pubkey,
    };

    let instruction = create_transfer_instruction(
        &rpc.get_payer().pubkey(),
        &payer.pubkey(),              // authority
        &Vec::new(),                  // input_compressed_account_merkle_tree_pubkeys
        &[output_compressed_account], // output_compressed_accounts
        &Vec::new(),                  // root_indices
        &None,
        &Vec::new(),                                                 // input_token_data
        &Vec::new(),                                                 // input_compressed_accounts
        *mint,                                                       // mint
        None,                                                        // owner_if_delegate_is_signer
        true,                                                        // is_compress
        Some(amount),                                                // compression_amount
        Some(get_token_pool_pda_with_index(mint, token_pool_index)), // token_pool_pda
        Some(*sender_token_account), // compress_or_decompress_token_account
        true,
        None,
        None,
        is_token_22,
        additional_pool_accounts.unwrap_or_default().as_slice(),
        false,
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
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &payer.pubkey(),
        &[&context_payer, payer],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());

    assert_transfer(
        rpc,
        test_indexer,
        &[output_compressed_account],
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        None,
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
pub async fn approve_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    delegated_amount: u64,
    delegate_lamports: Option<u64>,
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
        .map(|x| {
            x.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .into()
        })
        .collect::<Vec<_>>();
    println!(
        "input_compressed_account_hashes: {:?}",
        input_compressed_account_hashes
    );
    println!("input compressed accounts: {:?}", input_compressed_accounts);
    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
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
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.compressed_account.clone())
            .collect::<Vec<_>>(),
        mint,
        delegated_amount,
        delegate_lamports,
        delegated_compressed_account_merkle_tree: *delegated_compressed_account_merkle_tree,
        change_compressed_account_merkle_tree: *change_compressed_account_merkle_tree,
        delegate: *delegate,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof: proof_rpc_result.value.proof.0.unwrap_or_default(),
    };

    let instruction = create_approve_instruction(inputs).unwrap();
    let mut output_merkle_tree_pubkeys = vec![*delegated_compressed_account_merkle_tree];
    let input_amount = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let change_amount = input_amount - delegated_amount;
    let input_lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.lamports)
        .sum::<u64>();
    let (change_lamports, change_lamports_greater_zero) =
        if let Some(delegate_lamports) = delegate_lamports {
            let change_lamports = input_lamports - delegate_lamports;
            let option_change_lamports = if change_lamports > 0 {
                Some(change_lamports)
            } else {
                None
            };

            (
                Some(vec![Some(delegate_lamports), option_change_lamports]),
                change_lamports > 0,
            )
        } else if input_lamports > 0 {
            (Some(vec![None, Some(input_lamports)]), true)
        } else {
            (None, false)
        };
    if change_lamports_greater_zero || change_amount > 0 {
        output_merkle_tree_pubkeys.push(*change_compressed_account_merkle_tree);
    }
    let output_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(&output_merkle_tree_pubkeys);

    let output_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, output_merkle_tree_accounts.as_slice()).await;
    let input_merkle_tree_accounts =
        test_indexer.get_state_merkle_tree_accounts(input_merkle_tree_pubkeys.as_slice());
    let input_merkle_tree_test_snapshots =
        get_merkle_tree_snapshots::<R>(rpc, input_merkle_tree_accounts.as_slice()).await;
    let context_payer = rpc.get_payer().insecure_clone();
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &context_payer.pubkey(),
        &[&context_payer, authority],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());

    let expected_delegated_token_data = TokenData {
        mint: mint.into(),
        owner: authority.pubkey().into(),
        amount: delegated_amount,
        delegate: Some((*delegate).into()),
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };

    assert_eq!(
        expected_delegated_token_data,
        sdk_to_program_token_data(created_output_accounts[0].token_data.clone())
    );
    let mut expected_token_data = vec![expected_delegated_token_data];
    let mut delegates = vec![Some(*delegate)];
    if delegated_amount != input_amount {
        let expected_change_token_data = TokenData {
            mint: mint.into(),
            owner: authority.pubkey().into(),
            amount: change_amount,
            delegate: None,
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        assert_eq!(
            expected_change_token_data,
            sdk_to_program_token_data(created_output_accounts[1].token_data.clone())
        );
        expected_token_data.push(expected_change_token_data);
        delegates.push(None);
    }

    let expected_compressed_output_accounts =
        create_expected_token_output_data(expected_token_data, &output_merkle_tree_pubkeys);

    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        change_lamports,
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        Some(delegates),
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn revoke_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    output_account_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| {
            x.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .into()
        })
        .collect::<Vec<_>>();
    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
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
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.compressed_account.clone())
            .collect::<Vec<_>>(),
        mint,
        output_account_merkle_tree: *output_account_merkle_tree,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof: proof_rpc_result.value.proof.0.unwrap_or_default(),
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
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &context_payer.pubkey(),
        &[&context_payer, authority],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());
    let input_amount = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let expected_token_data = TokenData {
        mint: mint.into(),
        owner: authority.pubkey().into(),
        amount: input_amount,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: None,
    };
    assert_eq!(
        expected_token_data,
        sdk_to_program_token_data(created_output_accounts[0].token_data.clone())
    );
    let expected_compressed_output_accounts =
        create_expected_token_output_data(vec![expected_token_data], &output_merkle_tree_pubkeys);
    let sum_inputs = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.lamports)
        .sum::<u64>();
    let change_lamports = if sum_inputs > 0 {
        Some(vec![Some(sum_inputs)])
    } else {
        None
    };
    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        change_lamports,
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        None,
    )
    .await;
}

pub async fn freeze_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    freeze_or_thaw_test::<R, true, I>(
        authority,
        rpc,
        test_indexer,
        input_compressed_accounts,
        outputs_merkle_tree,
        transaction_params,
    )
    .await;
}

pub async fn thaw_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    freeze_or_thaw_test::<R, false, I>(
        authority,
        rpc,
        test_indexer,
        input_compressed_accounts,
        outputs_merkle_tree,
        transaction_params,
    )
    .await;
}

pub async fn freeze_or_thaw_test<
    R: Rpc + TestRpc + Indexer,
    const FREEZE: bool,
    I: Indexer + TestIndexerExtensions,
>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    outputs_merkle_tree: &Pubkey,
    transaction_params: Option<TransactionParams>,
) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| {
            x.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .into()
        })
        .collect::<Vec<_>>();
    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
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
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account.compressed_account)
            .cloned()
            .collect::<Vec<_>>(),
        outputs_merkle_tree: *outputs_merkle_tree,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof: proof_rpc_result.value.proof.0.unwrap_or_default(),
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
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &context_payer.pubkey(),
        &[&context_payer, authority],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());

    let mut delegates = Vec::new();
    let mut expected_output_accounts = Vec::new();
    for account in input_compressed_accounts.iter() {
        let state = if FREEZE {
            CompressedTokenAccountState::Frozen
        } else {
            CompressedTokenAccountState::Initialized
        };
        let expected_token_data = TokenData {
            mint: mint.into(),
            owner: input_compressed_accounts[0].token_data.owner.into(),
            amount: account.token_data.amount,
            delegate: account.token_data.delegate.map(|d| d.into()),
            state: state as u8,
            tlv: None,
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
    let sum_inputs = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.lamports)
        .sum::<u64>();
    let change_lamports = if sum_inputs > 0 {
        let mut change_lamports = Vec::new();
        for account in input_compressed_accounts.iter() {
            if account.compressed_account.compressed_account.lamports > 0 {
                change_lamports.push(Some(account.compressed_account.compressed_account.lamports));
            } else {
                change_lamports.push(None);
            }
        }
        Some(change_lamports)
    } else {
        None
    };
    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        change_lamports,
        input_compressed_account_hashes.as_slice(),
        &output_merkle_tree_test_snapshots,
        &input_merkle_tree_test_snapshots,
        &event,
        Some(delegates),
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn burn_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    change_account_merkle_tree: &Pubkey,
    burn_amount: u64,
    signer_is_delegate: bool,
    transaction_params: Option<TransactionParams>,
    is_token_22: bool,
    token_pool_index: u8,
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
        is_token_22,
        token_pool_index,
        None,
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

    let token_pool_pda_address = get_token_pool_pda_with_index(&mint, token_pool_index);
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
    let (event, _signature, _) = <R as TestRpc>::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &context_payer.pubkey(),
        &[&context_payer, authority],
        transaction_params,
    )
    .await
    .unwrap()
    .unwrap();
    let slot = rpc.get_slot().await.unwrap();
    let (_, created_output_accounts) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.clone());
    let mut delegates = Vec::new();
    let mut expected_output_accounts = Vec::new();

    let delegate = if signer_is_delegate {
        Some(authority.pubkey())
    } else {
        None
    };
    if output_amount > 0 {
        let expected_token_data = TokenData {
            mint: mint.into(),
            owner: input_compressed_accounts[0].token_data.owner.into(),
            amount: output_amount,
            delegate: delegate.map(|d| d.into()),
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        if let Some(delegate) = expected_token_data.delegate {
            delegates.push(Some(delegate.into()));
        } else {
            delegates.push(None);
        }
        expected_output_accounts.push(expected_token_data);
    }
    let expected_compressed_output_accounts =
        create_expected_token_output_data(expected_output_accounts, &output_merkle_tree_pubkeys);
    let sum_inputs = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.lamports)
        .sum::<u64>();
    let change_lamports = if sum_inputs > 0 {
        Some(vec![Some(sum_inputs)])
    } else {
        None
    };
    assert_transfer(
        rpc,
        test_indexer,
        expected_compressed_output_accounts.as_slice(),
        created_output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        change_lamports,
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
pub async fn create_burn_test_instruction<R: Rpc + Indexer, I: Indexer + TestIndexerExtensions>(
    authority: &Keypair,
    rpc: &mut R,
    test_indexer: &mut I,
    input_compressed_accounts: &[TokenDataWithMerkleContext],
    change_account_merkle_tree: &Pubkey,
    burn_amount: u64,
    signer_is_delegate: bool,
    mode: BurnInstructionMode,
    is_token_22: bool,
    token_pool_index: u8,
    additional_pool_accounts: Option<Vec<Pubkey>>,
) -> (Vec<[u8; 32]>, Vec<Pubkey>, Pubkey, u64, Instruction) {
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| {
            x.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .into()
        })
        .collect::<Vec<_>>();
    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await
        .unwrap();
    let mint = if mode == BurnInstructionMode::InvalidMint {
        Pubkey::new_unique()
    } else {
        input_compressed_accounts[0].token_data.mint
    };
    let proof = if mode == BurnInstructionMode::InvalidProof {
        CompressedProof {
            a: proof_rpc_result.value.proof.0.as_ref().unwrap().a,
            b: proof_rpc_result.value.proof.0.as_ref().unwrap().b,
            c: proof_rpc_result.value.proof.0.as_ref().unwrap().a, // flip c to make proof invalid but not run into decompress errors
        }
    } else {
        proof_rpc_result.value.proof.0.unwrap_or_default()
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
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account.compressed_account)
            .cloned()
            .collect::<Vec<_>>(),
        change_account_merkle_tree: *change_account_merkle_tree,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof,
        mint,
        signer_is_delegate,
        burn_amount,
        is_token_22,
        token_pool_index,
        additional_pool_accounts: additional_pool_accounts.unwrap_or_default(),
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
            owner: token_data.owner.into(),
            amount: token_data.amount,
            merkle_tree: *merkle_tree_pubkey,
            lamports: None,
        });
    }
    expected_compressed_output_accounts
}
