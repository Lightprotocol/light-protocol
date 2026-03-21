use std::mem;

use anchor_lang::prelude::*;
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_merkle_tree_metadata::fee::{compute_claimable_excess, hardcoded_rent_exemption};

use crate::{
    errors::AccountCompressionErrorCode,
    state::{
        address::{address_merkle_tree_from_bytes_zero_copy, AddressMerkleTreeAccount},
        public_state_merkle_tree::{
            state_merkle_tree_from_bytes_zero_copy, StateMerkleTreeAccount,
        },
    },
    utils::{
        check_signer_is_registered_or_authority::GroupAccess, transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct ClaimFees<'info> {
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: validated by owner check and discriminator dispatch in process_claim_fees.
    #[account(mut)]
    pub merkle_tree_or_queue: AccountInfo<'info>,
    /// CHECK: receives excess fees.
    #[account(mut)]
    pub fee_recipient: AccountInfo<'info>,
}

struct ClaimParams {
    rollover_fee: u64,
    capacity: u64,
    next_index: u64,
    data_len: u64,
}

/// Mirrors `manual_check_signer_is_registered_or_authority` from context.rs.
/// Uses `GroupAccess` trait rather than raw Pubkeys, so every account type
/// is validated through the same interface.
///
/// Extracted here because claim_fees dispatches on runtime-determined account
/// types, making the trait-based `GroupAccounts<Context>` approach impractical
/// (Anchor lifetime constraints prevent `AccountLoader::try_from` on dynamic
/// Context fields).
fn check_auth(
    derived_address: &Option<(Pubkey, Pubkey)>,
    authority_key: Pubkey,
    checked_account: &impl GroupAccess,
) -> Result<()> {
    match derived_address {
        Some((derived_addr, group_authority_pda)) => {
            if authority_key == *derived_addr
                && checked_account.get_owner() == *group_authority_pda
            {
                Ok(())
            } else {
                err!(AccountCompressionErrorCode::InvalidAuthority)
            }
        }
        None => {
            if authority_key == checked_account.get_owner() {
                Ok(())
            } else {
                err!(AccountCompressionErrorCode::InvalidAuthority)
            }
        }
    }
}

pub fn process_claim_fees(ctx: Context<ClaimFees>) -> Result<()> {
    // Owner check: only accounts owned by this program are valid targets.
    // V2 accounts also check via light-account-checks in *_from_account_info.
    // V1 accounts are parsed with bytemuck (no built-in owner check), so
    // this upfront check covers them.
    if ctx.accounts.merkle_tree_or_queue.owner != &crate::ID {
        return err!(AccountCompressionErrorCode::InvalidAccountType);
    }

    let derived_address = ctx
        .accounts
        .registered_program_pda
        .as_ref()
        .map(|rpda| (rpda.registered_program_signer_pda, rpda.group_authority_pda));

    let authority_key = ctx.accounts.authority.key();

    let discriminator = {
        let data = ctx.accounts.merkle_tree_or_queue.try_borrow_data()?;
        if data.len() < 8 {
            return err!(AccountCompressionErrorCode::InvalidAccountType);
        }
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&data[..8]);
        disc
    };

    let params = if discriminator
        == <BatchedMerkleTreeAccount as LightDiscriminator>::LIGHT_DISCRIMINATOR
    {
        let tree_type = {
            let data = ctx.accounts.merkle_tree_or_queue.try_borrow_data()?;
            if data.len() < 16 {
                return err!(AccountCompressionErrorCode::InvalidAccountType);
            }
            u64::from_le_bytes(data[8..16].try_into().unwrap())
        };
        let acct = &ctx.accounts.merkle_tree_or_queue;
        // state_from_account_info / address_from_account_info check owner + discriminator
        // via light-account-checks::check_account_info.
        let (rollover_fee, capacity, next_index) =
            if tree_type == light_merkle_tree_metadata::STATE_MERKLE_TREE_TYPE_V2 {
                let tree = BatchedMerkleTreeAccount::state_from_account_info(acct)
                    .map_err(ProgramError::from)?;
                check_auth(&derived_address, authority_key, &tree)?;
                let m = tree.get_metadata();
                (
                    m.metadata.rollover_metadata.rollover_fee,
                    m.capacity,
                    m.next_index,
                )
            } else if tree_type == light_merkle_tree_metadata::ADDRESS_MERKLE_TREE_TYPE_V2 {
                let tree = BatchedMerkleTreeAccount::address_from_account_info(acct)
                    .map_err(ProgramError::from)?;
                check_auth(&derived_address, authority_key, &tree)?;
                let m = tree.get_metadata();
                (
                    m.metadata.rollover_metadata.rollover_fee,
                    m.capacity,
                    m.next_index,
                )
            } else {
                return err!(AccountCompressionErrorCode::InvalidAccountType);
            };
        ClaimParams {
            rollover_fee,
            capacity,
            next_index,
            data_len: acct.data_len() as u64,
        }
    } else if discriminator == <BatchedQueueAccount as LightDiscriminator>::LIGHT_DISCRIMINATOR {
        // output_from_account_info checks owner + discriminator via light-account-checks.
        let queue =
            BatchedQueueAccount::output_from_account_info(&ctx.accounts.merkle_tree_or_queue)
                .map_err(ProgramError::from)?;
        check_auth(&derived_address, authority_key, &queue)?;
        let metadata = queue.get_metadata();
        ClaimParams {
            rollover_fee: metadata.metadata.rollover_metadata.rollover_fee,
            capacity: metadata.tree_capacity,
            next_index: metadata.batch_metadata.next_index,
            data_len: ctx.accounts.merkle_tree_or_queue.data_len() as u64,
        }
    } else if discriminator == StateMerkleTreeAccount::DISCRIMINATOR {
        // V1 state tree: owner already checked upfront, discriminator matched here.
        // Parsed via bytemuck (AccountLoader::try_from has lifetime issues in
        // dynamic dispatch from Context fields).
        let data = ctx.accounts.merkle_tree_or_queue.try_borrow_data()?;
        let data_len = data.len() as u64;
        let metadata_end = 8 + mem::size_of::<StateMerkleTreeAccount>();
        if data.len() < metadata_end {
            return err!(AccountCompressionErrorCode::InvalidAccountType);
        }
        let account: &StateMerkleTreeAccount = bytemuck::from_bytes(&data[8..metadata_end]);
        check_auth(&derived_address, authority_key, account)?;
        let rollover_fee = account.metadata.rollover_metadata.rollover_fee;
        let tree = state_merkle_tree_from_bytes_zero_copy(&data)?;
        let capacity = 1u64
            .checked_shl(tree.height as u32)
            .ok_or(AccountCompressionErrorCode::IntegerOverflow)?;
        ClaimParams {
            rollover_fee,
            capacity,
            next_index: tree.next_index() as u64,
            data_len,
        }
    } else if discriminator == AddressMerkleTreeAccount::DISCRIMINATOR {
        // V1 address tree: owner already checked upfront, discriminator matched here.
        let data = ctx.accounts.merkle_tree_or_queue.try_borrow_data()?;
        let data_len = data.len() as u64;
        let metadata_end = 8 + mem::size_of::<AddressMerkleTreeAccount>();
        if data.len() < metadata_end {
            return err!(AccountCompressionErrorCode::InvalidAccountType);
        }
        let account: &AddressMerkleTreeAccount = bytemuck::from_bytes(&data[8..metadata_end]);
        check_auth(&derived_address, authority_key, account)?;
        let rollover_fee = account.metadata.rollover_metadata.rollover_fee;
        let tree = address_merkle_tree_from_bytes_zero_copy(&data)?;
        let capacity = 1u64
            .checked_shl(tree.merkle_tree.height as u32)
            .ok_or(AccountCompressionErrorCode::IntegerOverflow)?;
        ClaimParams {
            rollover_fee,
            capacity,
            next_index: tree.merkle_tree.next_index() as u64,
            data_len,
        }
    } else {
        return err!(AccountCompressionErrorCode::InvalidAccountType);
    };

    let rent_exemption = hardcoded_rent_exemption(params.data_len)
        .ok_or(AccountCompressionErrorCode::IntegerOverflow)?;

    let excess = compute_claimable_excess(
        ctx.accounts.merkle_tree_or_queue.lamports(),
        rent_exemption,
        params.rollover_fee,
        params.capacity,
        params.next_index,
    )
    .unwrap_or(0);

    if excess == 0 {
        return Ok(());
    }

    transfer_lamports(
        &ctx.accounts.merkle_tree_or_queue,
        &ctx.accounts.fee_recipient,
        excess,
    )
}
