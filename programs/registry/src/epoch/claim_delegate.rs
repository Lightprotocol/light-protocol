use crate::errors::RegistryError;
use crate::selection::delegate::DelegateAccount;
use anchor_lang::prelude::*;

use super::claim_forester::CompressedForesterEpochAccountInput;

/// Sync Stake Account:
/// - syncs the virtual balance of accumulated stake rewards to the stake
///   account
/// - it does not sync the token stake account balance
/// - the token stake account balance must be fully synced to perform any
///   actions that move delegated stake
/// 1. input a vector of compressed forester epoch accounts
/// 2. Check that epoch of first compressed forester epoch account is less than
///    DelegateAccount.last_sync_epoch
/// 3. iterate over all compressed forester epoch accounts, increase
///    Account.stake_weight by rewards_earned in every step
/// 4. set DelegateAccount.last_sync_epoch to the epoch of the last compressed
///    forester epoch account
/// 5. prove inclusion of last hash in State merkle tree
pub fn sync_delegate_account_instruction(
    delegate_account: &mut DelegateAccount,
    compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
    hashed_forester_pubkey: [u8; 32],
    previous_hash: [u8; 32],
    // last_account_merkle_tree_pubkey: Pubkey,
    // last_account_leaf_index: u64,
    // inclusion_proof: CompressedProof,
    // root_index: u16,
) -> Result<()> {
    sync_delegate_account(
        delegate_account,
        compressed_forester_epoch_pdas,
        hashed_forester_pubkey,
        previous_hash,
    )?;
    // included for completeness
    // let last_compressed_forester_pda_hash = CompressedAccount {
    //     owner: crate::ID,
    //     lamports: 0,
    //     address: None,
    //     data: Some(CompressedAccountData {
    //         discriminator: [0, 0, 0, 0, 0, 0, 0, 1],
    //         data_hash: previous_hash,
    //         data: vec![],
    //     }),
    // }
    // .hash(&last_account_merkle_tree_pubkey, leaf_index)
    // .map_err(ProgramError::from)?;
    // let root = get_root(root_index);
    // verify_last_compressed_forester_pda_hash zkp_inclusion_proof(root, last_compressed_forester_pda_hash)?;
    Ok(())
}

pub fn sync_delegate_account(
    delegate_account: &mut DelegateAccount,
    compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
    hashed_forester_pubkey: [u8; 32],
    mut previous_hash: [u8; 32],
) -> Result<()> {
    if compressed_forester_epoch_pdas.is_empty() {
        return Ok(());
    }
    let last_sync_epoch = delegate_account.last_sync_epoch;
    if compressed_forester_epoch_pdas[0].epoch <= last_sync_epoch {
        return err!(RegistryError::StakeAccountAlreadySynced);
    }

    for compressed_forester_epoch_pda in compressed_forester_epoch_pdas.iter() {
        // Forester pubkey is not hashed thus we use a random value and hash offchain
        let compressed_forester_epoch_pda = compressed_forester_epoch_pda
            .into_compressed_forester_epoch_pda(previous_hash, crate::ID);
        previous_hash = compressed_forester_epoch_pda.hash(hashed_forester_pubkey)?;
        let get_staker_epoch_reward =
            compressed_forester_epoch_pda.get_reward(delegate_account.delegated_stake_weight);
        delegate_account.delegated_stake_weight += get_staker_epoch_reward;
        delegate_account.pending_token_amount += get_staker_epoch_reward;
    }
    delegate_account.last_sync_epoch = compressed_forester_epoch_pdas.iter().last().unwrap().epoch;
    Ok(())
}

/// Sync Token Account:
/// - syncs the user stake compressed token accounts with the pending token amount of
///   the stake account
/// Compress tokens from forester pool account to user delegate token account.
pub fn sync_token_account_instruction(
    // forester_token_pool_account: &mut AccountInfo,
    // user_delegate_compressed_token_account: &mut MockCompressedTokenAccount,
    user_delegate_account: &mut DelegateAccount,
) {
    // forester_token_pool_account.balance -= user_delegate_account.pending_token_amount;
    // user_delegate_compressed_token_account.balance += user_delegate_account.pending_token_amount;
    user_delegate_account.pending_token_amount = 0;
}
