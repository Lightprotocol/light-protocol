use account_compression::{
    address_merkle_tree_from_bytes_zero_copy_mut, context::AcpAccount,
    state_merkle_tree_from_bytes_zero_copy_mut, AddressMerkleTreeAccount, QueueAccount,
    StateMerkleTreeAccount,
};
use anchor_lang::{
    prelude::{AccountInfo, AccountLoader},
    solana_program::msg,
    Discriminator as AnchorDiscriminator, Key, ToAccountInfo,
};
use light_account_checks::discriminator::Discriminator;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_compressed_account::{hash_to_bn254_field_size_be, pubkey::Pubkey};
use light_merkle_tree_metadata::{QueueType, TreeType};

use crate::{
    context::{MerkleTreeContext, SystemContext},
    errors::SystemProgramError,
};

pub(crate) fn try_from_account_infos<'a, 'info>(
    account_infos: &'info [AccountInfo<'info>],
    context: &mut SystemContext<'info>,
) -> std::result::Result<Vec<AcpAccount<'a, 'info>>, SystemProgramError> {
    let mut accounts = Vec::with_capacity(account_infos.len());
    for (index, account_info) in (0u8..).zip(account_infos.iter()) {
        let account = try_from_account_info(account_info, context, index)?;
        accounts.push(account);
    }
    Ok(accounts)
}

#[inline(always)]
pub(crate) fn try_from_account_info<'a, 'info>(
    account_info: &'info AccountInfo<'info>,
    context: &mut SystemContext<'info>,
    index: u8,
) -> std::result::Result<AcpAccount<'a, 'info>, SystemProgramError> {
    let mut discriminator = [0u8; 8];
    {
        let data = account_info
            .try_borrow_data()
            .map_err(|_| SystemProgramError::InvalidAccount)?;

        if data.len() < 8 {
            return Ok(AcpAccount::Unknown());
        }
        discriminator.copy_from_slice(&data[..8]);
    }

    let (account, program_owner) = match discriminator {
        BatchedMerkleTreeAccount::DISCRIMINATOR => {
            let mut tree_type = [0u8; 8];
            tree_type.copy_from_slice(
                &account_info
                    .try_borrow_data()
                    .map_err(|_| SystemProgramError::InvalidAccount)?[8..16],
            );
            let tree_type = TreeType::from(u64::from_le_bytes(tree_type));
            match tree_type {
                TreeType::BatchedAddress => {
                    let tree =
                        BatchedMerkleTreeAccount::address_from_account_info(account_info).unwrap();
                    let program_owner = tree.metadata.access_metadata.program_owner;
                    // for batched trees we set the fee when setting the rollover fee.
                    Ok((AcpAccount::BatchedAddressTree(tree), program_owner))
                }
                TreeType::BatchedState => {
                    let tree =
                        BatchedMerkleTreeAccount::state_from_account_info(account_info).unwrap();
                    let program_owner = tree.metadata.access_metadata.program_owner;
                    Ok((AcpAccount::BatchedStateTree(tree), program_owner))
                }
                _ => {
                    msg!(
                        "Invalid batched tree type. {:?} pubkey: {}",
                        tree_type,
                        account_info.key()
                    );
                    Err(SystemProgramError::InvalidAccount)
                }
            }
        }
        BatchedQueueAccount::DISCRIMINATOR => {
            let queue = BatchedQueueAccount::output_from_account_info(account_info).unwrap();
            let program_owner = queue.metadata.access_metadata.program_owner;
            Ok((AcpAccount::OutputQueue(queue), program_owner))
        }
        StateMerkleTreeAccount::DISCRIMINATOR => {
            let program_owner = {
                let merkle_tree =
                    AccountLoader::<StateMerkleTreeAccount>::try_from(account_info).unwrap();
                let merkle_tree = merkle_tree.load().unwrap();
                context.set_network_fee(merkle_tree.metadata.rollover_metadata.network_fee, index);
                context.set_legacy_merkle_context(
                    index,
                    MerkleTreeContext {
                        rollover_fee: merkle_tree.metadata.rollover_metadata.rollover_fee,
                        hashed_pubkey: hash_to_bn254_field_size_be(&account_info.key().to_bytes())
                            .unwrap()
                            .0,
                    },
                );

                merkle_tree.metadata.access_metadata.program_owner
            };
            let merkle_tree = account_info.try_borrow_mut_data();
            if merkle_tree.is_err() {
                msg!("merkle_tree.is_err() {:?}", merkle_tree);
                return Err(SystemProgramError::InvalidAccount);
            }
            let merkle_tree = &mut merkle_tree.map_err(|_| SystemProgramError::InvalidAccount)?;
            let data_slice: &'info mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
            };
            Ok((
                AcpAccount::StateTree((
                    account_info.key(),
                    state_merkle_tree_from_bytes_zero_copy_mut(data_slice).unwrap(),
                )),
                program_owner,
            ))
        }
        AddressMerkleTreeAccount::DISCRIMINATOR => {
            let program_owner = {
                let merkle_tree =
                    AccountLoader::<AddressMerkleTreeAccount>::try_from(account_info).unwrap();
                let merkle_tree = merkle_tree.load().unwrap();
                context.set_address_fee(merkle_tree.metadata.rollover_metadata.network_fee, index);
                merkle_tree.metadata.access_metadata.program_owner
            };
            let mut merkle_tree = account_info
                .try_borrow_mut_data()
                .map_err(|_| SystemProgramError::InvalidAccount)?;
            let data_slice: &'info mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
            };
            Ok((
                AcpAccount::AddressTree((
                    account_info.key(),
                    address_merkle_tree_from_bytes_zero_copy_mut(data_slice).unwrap(),
                )),
                program_owner,
            ))
        }
        QueueAccount::DISCRIMINATOR => {
            let queue = AccountLoader::<QueueAccount>::try_from(account_info).unwrap();
            let queue = queue.load().unwrap();
            if queue.metadata.queue_type == QueueType::AddressQueue as u64 {
                context.set_legacy_merkle_context(
                    index,
                    MerkleTreeContext {
                        rollover_fee: queue.metadata.rollover_metadata.rollover_fee,
                        hashed_pubkey: [0u8; 32], // not used for address trees
                    },
                );

                let program_owner = queue.metadata.access_metadata.program_owner;
                Ok((
                    AcpAccount::AddressQueue(*account_info.key, account_info.to_account_info()),
                    program_owner,
                ))
            } else if queue.metadata.queue_type == QueueType::NullifierQueue as u64 {
                Ok((
                    AcpAccount::V1Queue(account_info.to_account_info()),
                    Pubkey::default(),
                ))
            } else {
                msg!(
                    "Invalid queue account {:?} type {}",
                    account_info.key,
                    queue.metadata.queue_type
                );
                Err(SystemProgramError::InvalidAccount)
            }
        }
        // Needed for compatibility with the token program.
        _ => Ok((AcpAccount::Unknown(), Pubkey::default())),
    }?;

    if let AcpAccount::Unknown() = account {
        return Ok(account);
    }
    if account_compression::ID != *account_info.owner {
        msg!("Invalid owner {:?}", account_info.owner);
        msg!("Pubkey {:?}", account_info.key());
        return Err(SystemProgramError::InvalidAccount);
    }

    if program_owner != Pubkey::default() {
        if let Some(invoking_program) = context.invoking_program_id {
            if invoking_program != program_owner.into() {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.program_owner {:?}",
                    invoking_program,
                    program_owner
                );
                return Err(SystemProgramError::InvalidMerkleTreeOwner);
            }
        } else {
            return Err(SystemProgramError::InvalidMerkleTreeOwner);
        }
    }

    Ok(account)
}
