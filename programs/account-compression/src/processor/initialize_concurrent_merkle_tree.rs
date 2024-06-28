use anchor_lang::prelude::*;
use light_utils::fee::compute_rollover_fee;

use crate::{
    state::StateMerkleTreeAccount, state_merkle_tree_from_bytes_zero_copy_init, AccessMetadata,
    RolloverMetadata,
};

#[allow(unused_variables)]
pub fn process_initialize_state_merkle_tree(
    merkle_tree_account_loader: &AccountLoader<'_, StateMerkleTreeAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
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
                if (rollover_fee * rollover_threshold * (2u64.pow(*height))) / 100
                    <= queue_rent + merkle_tree_rent
                {
                    msg!("rollover_fee: {}", rollover_fee);
                    msg!("rollover_threshold: {}", rollover_threshold);
                    msg!("height: {}", height);
                    msg!("merkle_tree_rent: {}", merkle_tree_rent);
                    msg!("queue_rent: {}", queue_rent);
                    msg!(
                        "((rollover_fee * rollover_threshold * (2u64.pow(height))) / 100): {} < {} rent",
                        ((rollover_fee * rollover_threshold * (2u64.pow(*height))) / 100), queue_rent + merkle_tree_rent
                    );
                    return err!(
                        crate::errors::AccountCompressionErrorCode::InsufficientRolloverFee
                    );
                }
                rollover_fee
            }
            None => 0,
        };

        merkle_tree.init(
            AccessMetadata::new(owner, program_owner),
            RolloverMetadata::new(
                index,
                rollover_fee,
                rollover_threshold,
                network_fee,
                close_threshold,
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
