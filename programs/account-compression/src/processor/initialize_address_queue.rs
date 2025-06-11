use anchor_lang::prelude::*;
use light_merkle_tree_metadata::{
    access::AccessMetadata,
    fee::compute_rollover_fee,
    rollover::{check_rollover_fee_sufficient, RolloverMetadata},
    QueueType,
};

use crate::state::{queue_from_bytes_zero_copy_init, QueueAccount};

pub fn process_initialize_address_queue<'info>(
    queue_account_info: &AccountInfo<'info>,
    queue_loader: &AccountLoader<'info, QueueAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
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
            )
            .map_err(ProgramError::from)?;
            msg!("address queue rollover_fee: {}", rollover_fee);
            rollover_fee
        } else {
            0
        };

        address_queue.init(
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
            associated_merkle_tree,
            QueueType::AddressV1,
        );
        msg!("address queue address_queue.init");

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
