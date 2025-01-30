use crate::{
    context::SystemContext,
    errors::SystemProgramError,
    instruction_data::ZNewAddressParamsPacked,
    sdk::address::{derive_address, derive_address_legacy},
};
use account_compression::{
    context::AcpAccount, errors::AccountCompressionErrorCode,
    insert_into_queues::AppendNullifyCreateAddressInputs,
};
use anchor_lang::prelude::*;

pub fn derive_new_addresses<'a, 'info>(
    new_address_params: &[ZNewAddressParamsPacked],
    num_input_compressed_accounts: usize,
    remaining_accounts: &'info [AccountInfo<'info>],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut AppendNullifyCreateAddressInputs<'_>,
    accounts: &[AcpAccount<'a, 'info>],
) -> Result<()> {
    let init_len = context.account_indices.len();
    let invoking_program_id_bytes = context
        .invoking_program_id
        .as_ref()
        .map(|invoking_program_id| invoking_program_id.to_bytes());
    new_address_params
        .iter()
        .enumerate()
        .try_for_each(|(i, new_address_params)| {
            let (address, rollover_fee) = match &accounts
                [new_address_params.address_merkle_tree_account_index as usize]
            {
                AcpAccount::AddressTree((pubkey, _)) => {
                    cpi_ix_data.addresses[i].queue_index = context.get_index_or_insert(
                        new_address_params.address_queue_account_index,
                        &remaining_accounts,
                    );
                    cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                        new_address_params.address_merkle_tree_account_index,
                        &remaining_accounts,
                    );
                    (
                        derive_address_legacy(&pubkey, &new_address_params.seed)
                            .map_err(ProgramError::from)?,
                        context
                            .legacy_merkle_context
                            .iter()
                            .find(|x| x.0 == new_address_params.address_merkle_tree_account_index)
                            .unwrap()
                            .1
                            .rollover_fee,
                    )
                }
                AcpAccount::BatchedAddressTree(tree) => {
                    let invoking_program_id_bytes =
                        if let Some(bytes) = invoking_program_id_bytes.as_ref() {
                            Ok(bytes)
                        } else {
                            err!(SystemProgramError::DeriveAddressError)
                        }?;
                    cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                        new_address_params.address_merkle_tree_account_index,
                        &remaining_accounts,
                    );

                    (
                        derive_address(
                            &new_address_params.seed,
                            &tree.pubkey().to_bytes(),
                            invoking_program_id_bytes,
                        ),
                        tree.metadata.rollover_metadata.network_fee,
                    )
                }
                _ => {
                    return err!(
                        AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
                    )
                }
            };

            // We are inserting addresses into two vectors to avoid unwrapping
            // the option in following functions.
            context.addresses[i + num_input_compressed_accounts] = Some(address);
            cpi_ix_data.addresses[i].address = address;

            context.set_rollover_fee(new_address_params.address_queue_account_index, rollover_fee);
            Ok(())
        })?;
    cpi_ix_data.num_address_appends = (context.account_indices.len() - init_len) as u8;

    Ok(())
}
