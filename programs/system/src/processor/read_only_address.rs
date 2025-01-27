use account_compression::context::AcpAccount;
use anchor_lang::prelude::*;
use light_utils::instruction::instruction_data_zero_copy::ZPackedReadOnlyAddress;

use crate::errors::SystemProgramError;

#[inline(always)]
pub fn verify_read_only_address_queue_non_inclusion<'a>(
    remaining_accounts: &mut [AcpAccount<'a, '_>],
    read_only_addresses: &'a [ZPackedReadOnlyAddress],
) -> Result<()> {
    if read_only_addresses.is_empty() {
        return Ok(());
    }
    for read_only_address in read_only_addresses.iter() {
        let merkle_tree = if let AcpAccount::BatchedAddressTree(tree) =
            &mut remaining_accounts[read_only_address.address_merkle_tree_account_index as usize]
        {
            tree
        } else {
            msg!(
                "Read only address account is not a BatchedAddressTree {:?}",
                read_only_address
            );
            return err!(SystemProgramError::InvalidAccount);
        };
        merkle_tree
            .check_input_queue_non_inclusion(&read_only_address.address)
            .map_err(|_| SystemProgramError::ReadOnlyAddressAlreadyExists)?;
    }
    Ok(())
}
