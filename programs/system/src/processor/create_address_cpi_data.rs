use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    instruction_data::{
        insert_into_queues::InsertIntoQueuesInstructionDataMut, traits::NewAddressParamsTrait,
    },
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    accounts::check_accounts::AcpAccount, context::SystemContext, errors::SystemProgramError,
    Result,
};

pub fn derive_new_addresses<'info, 'a, 'b: 'a, const ADDRESS_ASSIGNMENT: bool>(
    new_address_params: impl Iterator<Item = &'a (dyn NewAddressParamsTrait<'b> + 'a)>,
    remaining_accounts: &'info [AccountInfo],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
    accounts: &[AcpAccount<'info>],
) -> Result<()> {
    // Get invoking_program_id early and store if available
    let invoking_program_id_clone = context.invoking_program_id;
    let mut seq_index = 0;

    for (i, new_address_params) in new_address_params.enumerate() {
        let (address, rollover_fee) = match &accounts
            [new_address_params.address_merkle_tree_account_index() as usize]
        {
            AcpAccount::AddressTree((pubkey, _)) => {
                cpi_ix_data.addresses[i].queue_index = context.get_index_or_insert(
                    new_address_params.address_queue_index(),
                    remaining_accounts,
                );
                cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                    new_address_params.address_merkle_tree_account_index(),
                    remaining_accounts,
                );

                (
                    derive_address_legacy(&pubkey.into(), &new_address_params.seed())
                        .map_err(ProgramError::from)?,
                    context
                        .get_legacy_merkle_context(new_address_params.address_queue_index())
                        .unwrap()
                        .rollover_fee,
                )
            }
            AcpAccount::BatchedAddressTree(tree) => {
                // Use the cloned reference instead of borrowing context again
                let invoking_program_id_bytes = if let Some(ref bytes) = invoking_program_id_clone {
                    Ok(bytes)
                } else {
                    Err(SystemProgramError::DeriveAddressError)
                }?;

                cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                    new_address_params.address_merkle_tree_account_index(),
                    remaining_accounts,
                );

                context.set_address_fee(
                    tree.metadata.rollover_metadata.network_fee,
                    new_address_params.address_merkle_tree_account_index(),
                );

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
