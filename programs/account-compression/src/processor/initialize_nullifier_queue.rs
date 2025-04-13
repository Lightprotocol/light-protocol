use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_merkle_tree_metadata::{access::AccessMetadata, rollover::RolloverMetadata, QueueType};

use crate::{queue_from_bytes_zero_copy_init, QueueAccount};

pub fn process_initialize_nullifier_queue<'a, 'b, 'c: 'info, 'info>(
    nullifier_queue_account_info: AccountInfo<'info>,
    nullifier_queue_account_loader: &'a AccountLoader<'info, QueueAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    associated_merkle_tree: Pubkey,
    capacity: u16,
    sequence_threshold: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    network_fee: u64,
) -> Result<()> {
    {
        let mut nullifier_queue = nullifier_queue_account_loader.load_init()?;
        let rollover_meta_data = RolloverMetadata {
            index,
            rollover_threshold: rollover_threshold.unwrap_or_default(),
            close_threshold: close_threshold.unwrap_or(u64::MAX),
            rolledover_slot: u64::MAX,
            network_fee,
            // The rollover fee is charged at append with the Merkle tree. The
            // rollover that is defined in the Merkle tree is calculated to
            // rollover the tree, queue and cpi context account.
            rollover_fee: 0,
            additional_bytes: 0,
        };

        nullifier_queue.init(
            AccessMetadata {
                owner: owner.into(),
                program_owner: program_owner.unwrap_or_default().into(),
                forester: forester.unwrap_or_default().into(),
            },
            rollover_meta_data,
            associated_merkle_tree,
            QueueType::NullifierV1,
        );

        drop(nullifier_queue);
    }

    let nullifier_queue = nullifier_queue_account_info;
    let mut nullifier_queue = nullifier_queue.try_borrow_mut_data()?;
    unsafe {
        queue_from_bytes_zero_copy_init(
            &mut nullifier_queue,
            capacity.into(),
            sequence_threshold as usize,
        )
        .map_err(ProgramError::from)?;
    }
    Ok(())
}
