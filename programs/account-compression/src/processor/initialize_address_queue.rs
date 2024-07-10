use anchor_lang::prelude::*;
use light_utils::fee::compute_rollover_fee;

use crate::{
    state::{queue_from_bytes_zero_copy_init, QueueAccount},
    AccessMetadata, QueueType, RolloverMetadata,
};

pub fn process_initialize_address_queue<'info>(
    queue_account_info: &AccountInfo<'info>,
    queue_loader: &AccountLoader<'info, QueueAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    associated_merkle_tree: Pubkey,
    capacity: u16,
    sequence_threshold: u64,
    network_fee: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    height: u32,
    merkle_tree_rent: u64,
) -> Result<()> {
    {
        let mut address_queue = queue_loader.load_init()?;

        // Since the user doesn't interact with the address Merkle tree
        // directly, we need to charge a `rollover_fee` both for the queue and
        // Merkle tree.
        let queue_rent = queue_account_info.lamports();
        let rollover_fee = if let Some(rollover_threshold) = rollover_threshold {
            let rollover_fee = compute_rollover_fee(rollover_threshold, height, merkle_tree_rent)
                .map_err(ProgramError::from)?
                + compute_rollover_fee(rollover_threshold, height, queue_rent)
                    .map_err(ProgramError::from)?;
            check_rollover_fee_sufficient(
                rollover_fee,
                queue_rent,
                merkle_tree_rent,
                rollover_threshold,
                height,
            )?;
            msg!("address queue rollover_fee: {}", rollover_fee);
            rollover_fee
        } else {
            0
        };

        address_queue.init(
            AccessMetadata::new(owner, program_owner),
            RolloverMetadata::new(
                index,
                rollover_fee,
                rollover_threshold,
                network_fee,
                close_threshold,
            ),
            associated_merkle_tree,
            QueueType::AddressQueue,
        );

        drop(address_queue);
    }

    unsafe {
        queue_from_bytes_zero_copy_init(
            &mut queue_account_info.try_borrow_mut_data()?,
            capacity as usize,
            sequence_threshold as usize,
        )
        .map_err(ProgramError::from)?;
    }

    Ok(())
}

pub fn check_rollover_fee_sufficient(
    rollover_fee: u64,
    queue_rent: u64,
    merkle_tree_rent: u64,
    rollover_threshold: u64,
    height: u32,
) -> Result<()> {
    if rollover_fee != queue_rent + merkle_tree_rent
        && (rollover_fee * rollover_threshold * (2u64.pow(height))) / 100
            < queue_rent + merkle_tree_rent
    {
        msg!("rollover_fee: {}", rollover_fee);
        msg!("rollover_threshold: {}", rollover_threshold);
        msg!("height: {}", height);
        msg!("merkle_tree_rent: {}", merkle_tree_rent);
        msg!("queue_rent: {}", queue_rent);
        msg!(
            "((rollover_fee * rollover_threshold * (2u64.pow(height))) / 100): {} < {} rent",
            ((rollover_fee * rollover_threshold * (2u64.pow(height))) / 100),
            queue_rent + merkle_tree_rent
        );
        return err!(crate::errors::AccountCompressionErrorCode::InsufficientRolloverFee);
    }
    Ok(())
}
