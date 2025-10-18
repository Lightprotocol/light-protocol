use anchor_lang::AnchorSerialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_compressed_token::process_transfer::{get_cpi_authority_pda, TokenTransferOutputData};
use light_event::event::PublicTransactionEvent;
use light_program_test::indexer::TestIndexerExtensions;
use light_sdk::token::TokenDataWithMerkleContext;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

use crate::assert_compressed_tx::{
    assert_merkle_tree_after_tx, assert_nullifiers_exist_in_hash_sets,
    assert_public_transaction_event, MerkleTreeTestSnapShot,
};

/// General token tx assert:
/// 1. outputs created
/// 2. inputs nullified
/// 3. Public Transaction event emitted correctly
/// 4. Merkle tree was updated correctly
/// 5. TODO: Fees have been paid (after fee refactor)
/// 6. Check compression amount was transferred (outside of this function)
///    No addresses in token transactions
#[allow(clippy::too_many_arguments)]
pub async fn assert_transfer<R: Rpc, I: Indexer + TestIndexerExtensions>(
    context: &mut R,
    test_indexer: &mut I,
    out_compressed_accounts: &[TokenTransferOutputData],
    created_output_compressed_accounts: &[CompressedAccountWithMerkleContext],
    lamports: Option<Vec<Option<u64>>>,
    input_compressed_account_hashes: &[[u8; 32]],
    output_merkle_tree_snapshots: &[MerkleTreeTestSnapShot],
    input_merkle_tree_test_snapshots: &[MerkleTreeTestSnapShot],
    event: &PublicTransactionEvent,
    delegates: Option<Vec<Option<Pubkey>>>,
) {
    // CHECK 1
    assert_compressed_token_accounts::<R, I>(
        test_indexer,
        out_compressed_accounts,
        lamports,
        output_merkle_tree_snapshots,
        delegates,
    );
    // CHECK 2
    assert_nullifiers_exist_in_hash_sets(
        context,
        input_merkle_tree_test_snapshots,
        input_compressed_account_hashes,
    )
    .await;
    let vec;
    let input_compressed_account_hashes = if input_compressed_account_hashes.is_empty() {
        None
    } else {
        vec = input_compressed_account_hashes.to_vec();
        Some(&vec)
    };
    // CHECK 4
    let sequence_numbers =
        assert_merkle_tree_after_tx(context, output_merkle_tree_snapshots, test_indexer).await;
    // CHECK 3
    assert_public_transaction_event(
        event,
        input_compressed_account_hashes,
        output_merkle_tree_snapshots
            .iter()
            .map(|x| x.accounts)
            .collect::<Vec<_>>()
            .as_slice(),
        &created_output_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.leaf_index)
            .collect::<Vec<_>>(),
        None,
        false,
        None,
        sequence_numbers,
    );
}

pub fn assert_compressed_token_accounts<R: Rpc, I: Indexer + TestIndexerExtensions>(
    test_indexer: &mut I,
    out_compressed_accounts: &[TokenTransferOutputData],
    lamports: Option<Vec<Option<u64>>>,
    output_merkle_tree_snapshots: &[MerkleTreeTestSnapShot],
    delegates: Option<Vec<Option<Pubkey>>>,
) {
    let delegates = delegates.unwrap_or(vec![None; out_compressed_accounts.len()]);
    let mut tree = Pubkey::default();
    let mut index = 0;
    let output_lamports = lamports.unwrap_or(vec![None; out_compressed_accounts.len()]);
    println!("out_compressed_accounts {:?}", out_compressed_accounts);

    for (i, out_compressed_account) in out_compressed_accounts.iter().enumerate() {
        if output_merkle_tree_snapshots[i].accounts.merkle_tree != tree {
            tree = output_merkle_tree_snapshots[i].accounts.merkle_tree;
            index = 0;
        } else {
            index += 1;
        }
        let pos = test_indexer
            .get_token_compressed_accounts()
            .iter()
            .position(|x| {
                x.token_data.owner == out_compressed_account.owner
                    && x.token_data.amount == out_compressed_account.amount
                    && x.token_data.delegate == delegates[i]
            })
            .expect("transfer recipient compressed account not found in mock indexer");
        let transfer_recipient_token_compressed_account =
            test_indexer.get_token_compressed_accounts()[pos].clone();
        assert_eq!(
            transfer_recipient_token_compressed_account
                .token_data
                .amount,
            out_compressed_account.amount
        );
        assert_eq!(
            transfer_recipient_token_compressed_account.token_data.owner,
            out_compressed_account.owner
        );
        assert_eq!(
            transfer_recipient_token_compressed_account
                .token_data
                .delegate,
            delegates[i]
        );
        let transfer_recipient_compressed_account = transfer_recipient_token_compressed_account
            .compressed_account
            .clone();
        println!(
            "transfer_recipient_compressed_account {:?}",
            transfer_recipient_compressed_account
        );
        if i < output_lamports.len() {
            assert_eq!(
                transfer_recipient_compressed_account
                    .compressed_account
                    .lamports,
                output_lamports[i].unwrap_or(0)
            );
        } else if i != output_lamports.len() {
            // This check accounts for change accounts which are dynamically created onchain.
            panic!("lamports not found in output_lamports");
        }
        assert!(transfer_recipient_compressed_account
            .compressed_account
            .data
            .is_some());
        let mut data = Vec::new();
        transfer_recipient_token_compressed_account
            .token_data
            .serialize(&mut data)
            .unwrap();
        assert_eq!(
            transfer_recipient_compressed_account
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data,
            data
        );
        assert_eq!(
            transfer_recipient_compressed_account
                .compressed_account
                .owner
                .to_bytes(),
            light_compressed_token::ID.to_bytes()
        );

        if !test_indexer
            .get_token_compressed_accounts()
            .iter()
            .any(|x| {
                x.compressed_account.merkle_context.leaf_index as usize
                    == output_merkle_tree_snapshots[i].next_index + index
            })
        {
            println!(
                "token_compressed_accounts {:?}",
                test_indexer.get_token_compressed_accounts()
            );
            println!("snapshot {:?}", output_merkle_tree_snapshots[i]);
            println!("index {:?}", index);
            panic!("transfer recipient compressed account not found in mock indexer");
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn assert_mint_to<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    recipients: &[Pubkey],
    mint: Pubkey,
    amounts: &[u64],
    snapshots: &[MerkleTreeTestSnapShot],
    created_token_accounts: &[TokenDataWithMerkleContext],
    previous_mint_supply: u64,
    previous_sol_pool_amount: u64,
    token_pool_pda: Pubkey,
) {
    let mut created_token_accounts = created_token_accounts.to_vec();
    for (recipient, amount) in recipients.iter().zip(amounts) {
        let pos = created_token_accounts
            .iter()
            .position(|x| {
                x.token_data.owner == *recipient
                    && x.token_data.amount == *amount
                    && x.token_data.mint == mint
                    && x.token_data.delegate.is_none()
            })
            .expect("Mint to failed to create expected compressed token account.");
        created_token_accounts.remove(pos);
    }
    assert_merkle_tree_after_tx(rpc, snapshots, test_indexer).await;
    let mint_account: spl_token::state::Mint =
        spl_token::state::Mint::unpack(&rpc.get_account(mint).await.unwrap().unwrap().data)
            .unwrap();
    let sum_amounts = amounts.iter().sum::<u64>();
    assert_eq!(mint_account.supply, previous_mint_supply + sum_amounts);

    let pool_account = spl_token::state::Account::unpack(
        &rpc.get_account(token_pool_pda).await.unwrap().unwrap().data,
    )
    .unwrap();
    assert_eq!(pool_account.amount, previous_sol_pool_amount + sum_amounts);
}

pub async fn assert_create_mint<R: Rpc>(
    context: &mut R,
    authority: &Pubkey,
    mint: &Pubkey,
    pool: &Pubkey,
) {
    let mint_account: spl_token::state::Mint =
        spl_token::state::Mint::unpack(&context.get_account(*mint).await.unwrap().unwrap().data)
            .unwrap();
    assert_eq!(mint_account.supply, 0);
    assert_eq!(mint_account.decimals, 2);
    assert_eq!(mint_account.mint_authority.unwrap(), *authority);
    assert_eq!(mint_account.freeze_authority, Some(*authority).into());
    assert!(mint_account.is_initialized);
    let mint_account: spl_token::state::Account =
        spl_token::state::Account::unpack(&context.get_account(*pool).await.unwrap().unwrap().data)
            .unwrap();

    assert_eq!(mint_account.amount, 0);
    assert_eq!(mint_account.delegate, None.into());
    assert_eq!(mint_account.mint, *mint);
    assert_eq!(mint_account.owner, get_cpi_authority_pda().0);
}
