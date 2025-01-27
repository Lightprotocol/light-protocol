use account_compression::{context::AcpAccount, errors::AccountCompressionErrorCode};
use anchor_lang::prelude::*;
use light_utils::instruction::{
    address::{derive_address, derive_address_legacy},
    insert_into_queues::AppendNullifyCreateAddressInputs,
    instruction_data_zero_copy::ZNewAddressParamsPacked,
};

use crate::{context::SystemContext, errors::SystemProgramError};

pub fn derive_new_addresses<'info>(
    new_address_params: &[ZNewAddressParamsPacked],
    num_input_compressed_accounts: usize,
    remaining_accounts: &'info [AccountInfo<'info>],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut AppendNullifyCreateAddressInputs<'_>,
    accounts: &[AcpAccount<'_, 'info>],
) -> Result<()> {
    let invoking_program_id_bytes = context
        .invoking_program_id
        .as_ref()
        .map(|invoking_program_id| invoking_program_id.to_bytes());
    new_address_params
        .iter()
        .enumerate()
        .try_for_each(|(i, new_address_params)| {
            let (address, rollover_fee) =
                match &accounts[new_address_params.address_merkle_tree_account_index as usize] {
                    AcpAccount::AddressTree((pubkey, _)) => {
                        cpi_ix_data.addresses[i].queue_index = context.get_index_or_insert(
                            new_address_params.address_queue_account_index,
                            remaining_accounts,
                        );
                        cpi_ix_data.addresses[i].tree_index = context.get_index_or_insert(
                            new_address_params.address_merkle_tree_account_index,
                            remaining_accounts,
                        );
                        msg!(
                            "v1 address tree rollover fee {}",
                            context
                                .get_legacy_merkle_context(
                                    new_address_params.address_queue_account_index,
                                )
                                .unwrap()
                                .rollover_fee
                        );
                        (
                            derive_address_legacy(pubkey, &new_address_params.seed)
                                .map_err(ProgramError::from)?,
                            context
                                .get_legacy_merkle_context(
                                    new_address_params.address_queue_account_index,
                                )
                                .unwrap()
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
                            remaining_accounts,
                        );
                        context.set_address_fee(
                            tree.metadata.rollover_metadata.network_fee,
                            new_address_params.address_merkle_tree_account_index,
                        );
                        msg!(
                            "batched address tree fee {:?}",
                            tree.metadata.rollover_metadata.rollover_fee
                        );

                        (
                            derive_address(
                                &new_address_params.seed,
                                &tree.pubkey().to_bytes(),
                                invoking_program_id_bytes,
                            ),
                            tree.metadata.rollover_metadata.rollover_fee,
                        )
                    }
                    _ => {
                        return err!(
                        AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
                    )
                    }
                };
            msg!(
                "i + num_input_compressed_accounts: {:?}",
                i + num_input_compressed_accounts
            );
            msg!("address: {:?}", address);
            msg!("i: {:?}", i);
            msg!(
                "num_input_compressed_accounts: {:?}",
                num_input_compressed_accounts
            );
            // We are inserting addresses into two vectors to avoid unwrapping
            // the option in following functions.
            context.addresses.push(Some(address));
            cpi_ix_data.addresses[i].address = address;

            context.set_rollover_fee(new_address_params.address_queue_account_index, rollover_fee);
            Ok(())
        })?;
    cpi_ix_data.num_address_queues = accounts
        .iter()
        .filter(|x| {
            matches!(
                x,
                AcpAccount::AddressTree(_) | AcpAccount::BatchedAddressTree(_)
            )
        })
        .count() as u8;
    msg!(
        "cpi_ix_data.num_address_queues: {:?}",
        cpi_ix_data.num_address_queues
    );
    msg!(
        "start output appends: {:?}",
        cpi_ix_data.start_output_appends
    );
    Ok(())
}
