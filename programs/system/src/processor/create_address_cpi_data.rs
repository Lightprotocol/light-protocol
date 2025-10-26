use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    instruction_data::{
        insert_into_queues::InsertIntoQueuesInstructionDataMut, traits::NewAddress,
    },
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    accounts::remaining_account_checks::AcpAccount, context::SystemContext,
    errors::SystemProgramError, Result,
};

#[profile]
pub fn derive_new_addresses<'info, 'a, 'b: 'a, const ADDRESS_ASSIGNMENT: bool>(
    new_address_params: impl Iterator<Item = &'a (dyn NewAddress<'b> + 'a)>,
    remaining_accounts: &'info [AccountInfo],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
    accounts: &[AcpAccount<'info>],
) -> Result<()> {
    let invoking_program_id_clone = context.invoking_program_id;
    let mut seq_index = 0;

    for (i, new_address_params) in new_address_params.enumerate() {
        let (address, rollover_fee) = match &accounts
            .get(new_address_params.address_merkle_tree_account_index() as usize)
            .ok_or(SystemProgramError::AddressAssignedAccountIndexOutOfBounds)?
        {
            AcpAccount::AddressTree((pubkey, _)) => {
                cpi_ix_data.addresses[i].queue_index = context.get_index_or_insert(
                    new_address_params.address_queue_index(),
                    remaining_accounts,
                    "V1 address queue",
                )?;
                cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                    new_address_params.address_merkle_tree_account_index(),
                    remaining_accounts,
                    "V1 address tree",
                )?;

                let network_fee = context
                    .get_legacy_merkle_context(
                        new_address_params.address_merkle_tree_account_index(),
                    )
                    .ok_or(SystemProgramError::MissingLegacyMerkleContext)?
                    .network_fee;
                context.set_address_fee(network_fee, new_address_params.address_queue_index())?;

                (
                    derive_address_legacy(pubkey, &new_address_params.seed())
                        .map_err(ProgramError::from)?,
                    context
                        .get_legacy_merkle_context(new_address_params.address_queue_index())
                        .ok_or(SystemProgramError::MissingLegacyMerkleContext)?
                        .rollover_fee,
                )
            }
            AcpAccount::BatchedAddressTree(tree) => {
                let invoking_program_id_bytes = if let Some(bytes) = new_address_params.owner() {
                    Ok(bytes)
                } else if let Some(invoking_program_id_clone) = invoking_program_id_clone.as_ref() {
                    Ok(invoking_program_id_clone)
                } else {
                    Err(SystemProgramError::DeriveAddressError)
                }?;

                cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                    new_address_params.address_merkle_tree_account_index(),
                    remaining_accounts,
                    "V2 address tree",
                )?;

                context.set_address_fee(
                    tree.metadata.rollover_metadata.network_fee,
                    new_address_params.address_merkle_tree_account_index(),
                )?;

                cpi_ix_data.insert_address_sequence_number(
                    &mut seq_index,
                    tree.pubkey(),
                    tree.queue_batches.next_index,
                );

                (
                    derive_address(
                        &new_address_params.seed(),
                        &tree.pubkey().to_bytes(),
                        invoking_program_id_bytes,
                    ),
                    tree.metadata.rollover_metadata.rollover_fee,
                )
            }
            _ => {
                return Err(ProgramError::from(
                    SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch,
                ))
            }
        };
        if !ADDRESS_ASSIGNMENT {
            // We are inserting addresses into two vectors to avoid unwrapping
            // the option in following functions.
            context.addresses.push(Some(address));
        } else if new_address_params
            .assigned_compressed_account_index()
            .is_some()
        {
            // Only addresses assigned to output accounts can be used in output accounts.
            context.addresses.push(Some(address));
        }
        cpi_ix_data.addresses[i].address = address;

        context.set_rollover_fee(new_address_params.address_queue_index(), rollover_fee);
    }
    cpi_ix_data.num_address_queues = accounts
        .iter()
        .filter(|x| {
            matches!(
                x,
                AcpAccount::AddressTree(_) | AcpAccount::BatchedAddressTree(_)
            )
        })
        .count() as u8;

    Ok(())
}
