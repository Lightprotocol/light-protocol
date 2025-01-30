use account_compression::context::AcpAccount;
use anchor_lang::prelude::*;

use crate::{errors::SystemProgramError, instruction_data::ZPackedReadOnlyAddress};

#[inline(always)]
pub fn verify_read_only_address_queue_non_inclusion<'a, 'info>(
    remaining_accounts: &mut [AcpAccount<'a, 'info>],
    read_only_addresses: &'a [ZPackedReadOnlyAddress],
) -> Result<()> {
    if read_only_addresses.is_empty() {
        return Ok(());
    }
    for read_only_address in read_only_addresses.iter() {
        let merkle_tree = if let AcpAccount::BatchedStateTree(tree) =
            &mut remaining_accounts[read_only_address.address_merkle_tree_account_index as usize]
        {
            tree
        } else {
            return err!(SystemProgramError::InvalidAccount);
        };
        merkle_tree
            .check_input_queue_non_inclusion(&read_only_address.address)
            .map_err(|_| SystemProgramError::ReadOnlyAddressAlreadyExists)?;
    }
    Ok(())
}
