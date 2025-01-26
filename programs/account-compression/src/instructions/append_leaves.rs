use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Discriminator};
use light_batched_merkle_tree::queue::{BatchedQueueAccount, BatchedQueueMetadata};
use light_hasher::Discriminator as HasherDiscriminator;

use crate::{
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
        },
        transfer_lamports::transfer_lamports_cpi,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct AppendLeaves<'info> {
    #[account(mut)]
    /// Fee payer pays rollover fee.
    pub fee_payer: Signer<'info>,
    /// Checked whether instruction is accessed by a registered program or owner = authority.
    pub authority: Signer<'info>,
    /// Some assumes that the Merkle trees are accessed by a registered program.
    /// None assumes that the Merkle trees are accessed by its owner.
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    pub system_program: Program<'info, System>,
}

impl GroupAccess for StateMerkleTreeAccount {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata.access_metadata.program_owner.into()
    }
}

impl GroupAccess for BatchedQueueMetadata {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata.access_metadata.program_owner.into()
    }
}

impl<'info> GroupAccounts<'info> for AppendLeaves<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ZeroOutLeafIndex {
    pub tree_index: u8,
    pub batch_index: u8,
    pub leaf_index: u16,
}

/// Perform batch appends to Merkle trees provided as remaining accounts. Leaves
/// are assumed to be ordered by Merkle tree account.
/// 1. Iterate over all remaining accounts (Merkle tree accounts)
/// 2. get first leaves that points to current Merkle tree account
/// 3. get last leaf that points to current Merkle tree account
/// 4. append batch to Merkle tree or insert into output queue
///     based on discriminator
/// 5. transfer rollover fee
/// 6. check if all leaves are processed
///     return Ok(()) if all leaves are processed
pub fn process_append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: Vec<(u8, [u8; 32])>,
) -> Result<()> {
    let mut leaves_processed: usize = 0;
    let len = ctx.remaining_accounts.len();
    // 1. Iterate over all remaining accounts (Merkle tree or output queue accounts)
    for i in 0..len {
        let merkle_tree_acc_info = &ctx.remaining_accounts[i];

        let rollover_fee: u64 = {
            // 2. get first leaves that points to current Merkle tree account
            let start = match leaves.iter().position(|x| x.0 as usize == i) {
                Some(pos) => Ok(pos),
                None => err!(AccountCompressionErrorCode::NoLeavesForMerkleTree),
            }?;
            // 3. get last leaf that points to current Merkle tree account
            let end = match leaves[start..].iter().position(|x| x.0 as usize != i) {
                Some(pos) => pos + start,
                None => leaves.len(),
            };
            let batch_size = end - start;
            leaves_processed += batch_size;

            //TODO: check whether copy from slice is more efficient
            let merkle_tree_acc_discriminator: [u8; 8] = ctx.remaining_accounts[i]
                .try_borrow_data()?[0..8]
                .try_into()
                .unwrap();
            // 4. append batch to Merkle tree or insert into output queue
            match merkle_tree_acc_discriminator {
                StateMerkleTreeAccount::DISCRIMINATOR => append_to_concurrent_merkle_tree(
                    &ctx,
                    merkle_tree_acc_info,
                    batch_size,
                    leaves[start..end]
                        .iter()
                        .map(|x| &x.1)
                        .collect::<Vec<&[u8; 32]>>()
                        .as_slice(),
                )?,
                BatchedQueueAccount::DISCRIMINATOR => insert_into_output_queue(
                    &ctx,
                    merkle_tree_acc_info,
                    batch_size,
                    &leaves[start..end],
                )?,
                _ => {
                    return err!(
                        AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                    )
                }
            }
        };
        // 5. transfer rollover fee
        transfer_lamports_cpi(&ctx.accounts.fee_payer, merkle_tree_acc_info, rollover_fee)?;
    }
    // 6. check if all leaves are processed
    if leaves_processed != leaves.len() {
        err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed)
    } else {
        Ok(())
    }
}

/// Append a batch of leaves to a concurrent Merkle tree.
/// 1. Check StateMerkleTreeAccount discriminator and ownership (AccountLoader)
/// 2. Check signer is registered or authority
/// 3. Append leaves to Merkle tree
/// 4. Return rollover fee
fn append_to_concurrent_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    merkle_tree_acc_info: &'info AccountInfo<'info>,
    batch_size: usize,
    leaves: &[&[u8; 32]],
) -> Result<u64> {
    let rollover_fee = {
        let merkle_tree_account =
            AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
                .map_err(ProgramError::from)?;

        {
            let merkle_tree_account = merkle_tree_account.load()?;
            let rollover_fee =
                merkle_tree_account.metadata.rollover_metadata.rollover_fee * batch_size as u64;

            check_signer_is_registered_or_authority::<AppendLeaves, StateMerkleTreeAccount>(
                ctx,
                &merkle_tree_account,
            )?;

            rollover_fee
        }
    };
    let mut merkle_tree = merkle_tree_acc_info.try_borrow_mut_data()?;
    let mut merkle_tree = state_merkle_tree_from_bytes_zero_copy_mut(&mut merkle_tree)?;
    merkle_tree
        .append_batch(leaves)
        .map_err(ProgramError::from)?;
    Ok(rollover_fee)
}

/// Insert a batch of leaves into a batched Merkle tree output queue.
/// 1. Check BatchedQueueAccount discriminator and ownership
///     (output_from_account_info)
/// 2. Check signer is registered or authority
/// 3. Insert leaves into output queue
/// 4. Return rollover fee
fn insert_into_output_queue<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    merkle_tree_acc_info: &'info AccountInfo<'info>,
    batch_size: usize,
    leaves: &[(u8, [u8; 32])],
) -> Result<u64> {
    let output_queue = &mut BatchedQueueAccount::output_from_account_info(merkle_tree_acc_info)
        .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<AppendLeaves, BatchedQueueMetadata>(
        ctx,
        output_queue,
    )?;

    for (_, leaf) in leaves {
        output_queue
            .insert_into_current_batch(leaf)
            .map_err(ProgramError::from)?;
    }

    let rollover_fee = output_queue.metadata.rollover_metadata.rollover_fee * batch_size as u64;
    Ok(rollover_fee)
}
