use light_account_checks::{checks::check_owner, discriminator::Discriminator};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_compressed_account::{
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, ADDRESS_MERKLE_TREE_ACCOUNT_DISCRIMINATOR,
        QUEUE_ACCOUNT_DISCRIMINATOR, STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR,
    },
    hash_to_bn254_field_size_be,
    pubkey::Pubkey,
    QueueType, TreeType,
};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopyMut;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopyMut;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, msg};

use crate::{
    account_compression_state::{
        address::{address_merkle_tree_from_bytes_zero_copy_mut, AddressMerkleTreeAccount},
        queue::QueueAccount,
        state::{state_merkle_tree_from_bytes_zero_copy_mut, StateMerkleTreeAccount},
    },
    context::{MerkleTreeContext, SystemContext},
    errors::SystemProgramError,
};

/// AccountCompressionProgramAccount
pub enum AcpAccount<'info> {
    Authority(&'info AccountInfo),
    RegisteredProgramPda(&'info AccountInfo),
    SystemProgram(&'info AccountInfo),
    OutputQueue(BatchedQueueAccount<'info>),
    BatchedStateTree(BatchedMerkleTreeAccount<'info>),
    BatchedAddressTree(BatchedMerkleTreeAccount<'info>),
    StateTree((Pubkey, ConcurrentMerkleTreeZeroCopyMut<'info, Poseidon, 26>)),
    AddressTree(
        (
            Pubkey,
            IndexedMerkleTreeZeroCopyMut<'info, Poseidon, usize, 26, 16>,
        ),
    ),
    AddressQueue(Pubkey, &'info AccountInfo),
    V1Queue(&'info AccountInfo),
    Unknown(),
}

#[profile]
pub(crate) fn try_from_account_infos<'info>(
    account_infos: &'info [AccountInfo],
    context: &mut SystemContext<'info>,
) -> std::result::Result<Vec<AcpAccount<'info>>, SystemProgramError> {
    let mut accounts = Vec::with_capacity(account_infos.len());
    for (index, account_info) in (0u8..).zip(account_infos.iter()) {
        let account = try_from_account_info(account_info, context, index)?;
        accounts.push(account);
    }
    Ok(accounts)
}

#[inline(always)]
#[profile]
pub(crate) fn try_from_account_info<'a, 'info: 'a>(
    account_info: &'info AccountInfo,
    context: &mut SystemContext<'info>,
    index: u8,
) -> std::result::Result<AcpAccount<'info>, SystemProgramError> {
    let mut discriminator = [0u8; 8];
    {
        let data = account_info
            .try_borrow_data()
            .map_err(|_| SystemProgramError::BorrowingDataFailed)?;

        if data.len() < 8 {
            return Ok(AcpAccount::Unknown());
        }
        discriminator.copy_from_slice(&data[..8]);
    }

    let (account, program_owner) = match discriminator {
        BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR => {
            let mut tree_type = [0u8; 8];
            tree_type.copy_from_slice(
                &account_info
                    .try_borrow_data()
                    .map_err(|_| SystemProgramError::BorrowingDataFailed)?[8..16],
            );
            let tree_type = TreeType::from(u64::from_le_bytes(tree_type));
            match tree_type {
                TreeType::AddressV2 => {
                    let tree = BatchedMerkleTreeAccount::address_from_account_info(account_info)?;
                    let program_owner = tree.metadata.access_metadata.program_owner;
                    // for batched trees we set the fee when setting the rollover fee.
                    Ok((AcpAccount::BatchedAddressTree(tree), program_owner))
                }
                TreeType::StateV2 => {
                    let tree = BatchedMerkleTreeAccount::state_from_account_info(account_info)?;
                    let program_owner = tree.metadata.access_metadata.program_owner;
                    Ok((AcpAccount::BatchedStateTree(tree), program_owner))
                }
                _ => {
                    msg!(format!(
                        "Invalid batched tree type. {:?} pubkey: {:?}",
                        tree_type,
                        account_info.key()
                    )
                    .as_str());
                    Err(SystemProgramError::InvalidAccount)
                }
            }
        }
        BatchedQueueAccount::LIGHT_DISCRIMINATOR => {
            let queue = BatchedQueueAccount::output_from_account_info(account_info)?;
            let program_owner = queue.metadata.access_metadata.program_owner;
            Ok((AcpAccount::OutputQueue(queue), program_owner))
        }
        STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR => {
            let program_owner = {
                check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)?;
                let data = account_info
                    .try_borrow_data()
                    .map_err(|_| SystemProgramError::BorrowingDataFailed)?;
                let merkle_tree = bytemuck::from_bytes::<StateMerkleTreeAccount>(
                    &data[8..StateMerkleTreeAccount::LEN],
                );
                context.set_legacy_merkle_context(
                    index,
                    MerkleTreeContext {
                        rollover_fee: merkle_tree.metadata.rollover_metadata.rollover_fee,
                        hashed_pubkey: hash_to_bn254_field_size_be(account_info.key().as_slice()),
                        network_fee: merkle_tree.metadata.rollover_metadata.network_fee,
                    },
                );

                merkle_tree.metadata.access_metadata.program_owner
            };
            let merkle_tree = account_info.try_borrow_mut_data();
            if merkle_tree.is_err() {
                return Err(SystemProgramError::InvalidAccount);
            }
            let merkle_tree = &mut merkle_tree.map_err(|_| SystemProgramError::InvalidAccount)?;
            // SAFETY: merkle_tree is a valid RefMut<[u8]>, pointer and length are valid
            let data_slice: &'info mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
            };
            Ok((
                AcpAccount::StateTree((
                    (*account_info.key()).into(),
                    state_merkle_tree_from_bytes_zero_copy_mut(data_slice)
                        .map_err(|e| SystemProgramError::ProgramError(e.into()))?,
                )),
                program_owner,
            ))
        }
        ADDRESS_MERKLE_TREE_ACCOUNT_DISCRIMINATOR => {
            let program_owner = {
                check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)?;
                let data = account_info
                    .try_borrow_data()
                    .map_err(|_| SystemProgramError::BorrowingDataFailed)?;

                let merkle_tree = bytemuck::from_bytes::<AddressMerkleTreeAccount>(
                    &data[8..AddressMerkleTreeAccount::LEN],
                );
                context.set_legacy_merkle_context(
                    index,
                    MerkleTreeContext {
                        rollover_fee: merkle_tree.metadata.rollover_metadata.rollover_fee,
                        hashed_pubkey: [0u8; 32], // not used for address trees
                        network_fee: merkle_tree.metadata.rollover_metadata.network_fee,
                    },
                );
                merkle_tree.metadata.access_metadata.program_owner
            };
            let mut merkle_tree = account_info
                .try_borrow_mut_data()
                .map_err(|_| SystemProgramError::InvalidAccount)?;
            // SAFETY: merkle_tree is a valid RefMut<[u8]>, pointer and length are valid
            let data_slice: &'info mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
            };
            Ok((
                AcpAccount::AddressTree((
                    (*account_info.key()).into(),
                    address_merkle_tree_from_bytes_zero_copy_mut(data_slice)
                        .map_err(|e| SystemProgramError::ProgramError(e.into()))?,
                )),
                program_owner,
            ))
        }
        QUEUE_ACCOUNT_DISCRIMINATOR => {
            check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)?;
            let data = account_info
                .try_borrow_data()
                .map_err(|_| SystemProgramError::BorrowingDataFailed)?;
            let queue = bytemuck::from_bytes::<QueueAccount>(&data[8..QueueAccount::LEN]);

            if queue.metadata.queue_type == QueueType::AddressV1 as u64 {
                context.set_legacy_merkle_context(
                    index,
                    MerkleTreeContext {
                        rollover_fee: queue.metadata.rollover_metadata.rollover_fee,
                        hashed_pubkey: [0u8; 32], // not used for address trees
                        network_fee: queue.metadata.rollover_metadata.network_fee,
                    },
                );

                let program_owner = queue.metadata.access_metadata.program_owner;
                Ok((
                    AcpAccount::AddressQueue((*account_info.key()).into(), account_info),
                    program_owner,
                ))
            } else if queue.metadata.queue_type == QueueType::NullifierV1 as u64 {
                Ok((AcpAccount::V1Queue(account_info), Pubkey::default()))
            } else {
                msg!(format!(
                    "Invalid queue account {:?} type {}",
                    account_info.key(),
                    queue.metadata.queue_type
                )
                .as_str());
                Err(SystemProgramError::InvalidAccount)
            }
        }
        // Needed for compatibility with the token program.
        _ => Ok((AcpAccount::Unknown(), Pubkey::default())),
    }?;

    if let AcpAccount::Unknown() = account {
        return Ok(account);
    }
    if !account_info.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
        msg!(format!("Pubkey {:?}", account_info.key()).as_str());
        return Err(SystemProgramError::InvalidAccount);
    }

    if program_owner != Pubkey::default() {
        if let Some(invoking_program) = context.invoking_program_id {
            if invoking_program != program_owner.to_bytes() {
                msg!(format!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.program_owner {:?}",
                    invoking_program, program_owner
                )
                .as_str());
                return Err(SystemProgramError::InvalidMerkleTreeOwner);
            }
        } else {
            return Err(SystemProgramError::InvalidMerkleTreeOwner);
        }
    }

    Ok(account)
}
