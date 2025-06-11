use anchor_lang::prelude::*;
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    fee::compute_rollover_fee,
    rollover::{check_rollover_fee_sufficient, RolloverMetadata},
};

use crate::{state::StateMerkleTreeAccount, state_merkle_tree_from_bytes_zero_copy_init};

#[allow(unused_variables)]
pub fn process_initialize_state_merkle_tree(
    merkle_tree_account_loader: &AccountLoader<'_, StateMerkleTreeAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    height: &u32,
    changelog_size: &u64,
    roots_size: &u64,
    canopy_depth: &u64,
    associated_queue: Pubkey,
    network_fee: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    merkle_tree_rent: u64,
    queue_rent: u64,
) -> Result<()> {
    // Initialize new Merkle trees.
    {
        let mut merkle_tree = merkle_tree_account_loader.load_init()?;

        let rollover_fee = match rollover_threshold {
            Some(rollover_threshold) => {
                let rollover_fee =
                    compute_rollover_fee(rollover_threshold, *height, merkle_tree_rent)
                        .map_err(ProgramError::from)?
                        + compute_rollover_fee(rollover_threshold, *height, queue_rent)
                            .map_err(ProgramError::from)?;
                check_rollover_fee_sufficient(
                    rollover_fee,
                    queue_rent,
                    merkle_tree_rent,
                    rollover_threshold,
                    *height,
                )
                .map_err(ProgramError::from)?;
                msg!(" state Merkle tree rollover_fee: {}", rollover_fee);
                rollover_fee
            }
            None => 0,
        };

        merkle_tree.init(
            AccessMetadata {
                owner: owner.into(),
                program_owner: program_owner.unwrap_or_default().into(),
                forester: forester.unwrap_or_default().into(),
            },
            RolloverMetadata::new(
                index,
                rollover_fee,
                rollover_threshold,
                network_fee,
                close_threshold,
                None,
            ),
            associated_queue,
        );
    }

    let merkle_tree = merkle_tree_account_loader.to_account_info();
    let mut merkle_tree = merkle_tree.try_borrow_mut_data()?;
    let mut merkle_tree = state_merkle_tree_from_bytes_zero_copy_init(
        &mut merkle_tree,
        *height as usize,
        *canopy_depth as usize,
        *changelog_size as usize,
        *roots_size as usize,
    )?;
    merkle_tree.init().map_err(ProgramError::from)?;

    Ok(())
}
