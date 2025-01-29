use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_batched_merkle_tree::queue::BatchedQueueAccount;

use crate::{
    context::AcpAccount,
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
    utils::check_signer_is_registered_or_authority::{GroupAccess, GroupAccounts},
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

impl<'a> GroupAccess for BatchedQueueAccount<'a> {
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

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[repr(C)]
#[derive(
    KnownLayout,
    IntoBytes,
    Immutable,
    Copy,
    Clone,
    FromBytes,
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Debug,
    Unaligned,
)]
pub struct AppendLeavesInput {
    pub index: u8,
    pub leaf: [u8; 32],
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
    // ctx: &Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
    leaves: &[AppendLeavesInput],
    num_unique_appends: u8,
    accounts: &mut [AcpAccount<'a, 'info>],
) -> Result<()> {
    if leaves.is_empty() {
        return Ok(());
    }
    let mut leaves_processed: usize = 0;
    // 1. Iterate over all remaining accounts (Merkle tree or output queue accounts)
    for i in 0..num_unique_appends as usize {
        // 2. get first leaves that points to current Merkle tree account
        let start = match leaves.iter().position(|x| x.index as usize == i) {
            Some(pos) => Ok(pos),
            None => err!(AccountCompressionErrorCode::NoLeavesForMerkleTree),
        }?;
        // 3. get last leaf that points to current Merkle tree account
        let end = match leaves[start..].iter().position(|x| x.index as usize != i) {
            Some(pos) => pos + start,
            None => leaves.len(),
        };
        let batch_size = end - start;
        leaves_processed += batch_size;

        // 4. append batch to Merkle tree or insert into output queue
        match &mut accounts[i] {
            AcpAccount::StateTree((_, merkle_tree)) => {
                merkle_tree
                    .append_batch(
                        &leaves[start..end]
                            .iter()
                            .map(|x| &x.leaf)
                            .collect::<Vec<&[u8; 32]>>(),
                    )
                    .map_err(ProgramError::from)?;
            }
            AcpAccount::OutputQueue(queue) => {
                for leaf in leaves[start..end].iter() {
                    queue
                        .insert_into_current_batch(&leaf.leaf)
                        .map_err(ProgramError::from)?;
                }
            }
            _ => {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                )
            }
        }

        // 5. transfer rollover fee (rollover fee is transferred in the system program)
        // transfer_lamports_cpi(&accounts[0], merkle_tree_acc_info, rollover_fee)?;
    }
    // 6. check if all leaves are processed
    if leaves_processed != leaves.len() {
        err!(crate::errors::AccountCompressionErrorCode::NotAllLeavesProcessed)
    } else {
        Ok(())
    }
}

// /// Append a batch of leaves to a concurrent Merkle tree.
// /// 1. Check StateMerkleTreeAccount discriminator and ownership (AccountLoader)
// /// 2. Check signer is registered or authority
// /// 3. Append leaves to Merkle tree
// /// 4. Return rollover fee
// fn append_to_concurrent_merkle_tree<'a, 'b, 'c: 'info, 'info>(
//     merkle_tree_acc_info: ConcurrentMerkleTree26<'info>,
//     batch_size: usize,
//     leaves: &[&[u8; 32]],
// ) -> Result<()> {
//     // let rollover_fee = {
//     //     let merkle_tree_account =
//     //         AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
//     //             .map_err(ProgramError::from)?;

//     //     {
//     //         let merkle_tree_account = merkle_tree_account.load()?;
//     //         let rollover_fee =
//     //             merkle_tree_account.metadata.rollover_metadata.rollover_fee * batch_size as u64;

//     //         check_signer_is_registered_or_authority::<AppendLeaves, StateMerkleTreeAccount>(
//     //             ctx,
//     //             &merkle_tree_account,
//     //         )?;

//     //         rollover_fee
//     //     }
//     // };

//     merkle_tree
//         .append_batch(leaves)
//         .map_err(ProgramError::from)?;
//     Ok()
// }
