use account_compression::state_merkle_tree_from_bytes_zero_copy;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;

use crate::error::LightSdkError;
pub mod v1 {
    use light_account_checks::checks::check_owner;
    use light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID;

    use super::*;

    /// StateMerkleTreeAccount discriminator
    pub const STATE_MERKLE_TREE_DISCRIMINATOR: [u8; 8] = [172, 43, 172, 186, 29, 73, 219, 84];

    /// Reads a root from the concurrent state merkle tree by index
    pub fn read_state_merkle_tree_root(
        account_info: &AccountInfo,
        root_index: u16,
    ) -> Result<[u8; 32], LightSdkError> {
        if root_index as usize >= 2400 {
            msg!(
                "Invalid root index: {} greater than max root index {}",
                root_index,
                2400
            );
            return Err(LightSdkError::from(ProgramError::InvalidArgument));
        }
        check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)?;
        let account_data = account_info.try_borrow_data()?;

        // Check discriminator
        if account_data.len() < 8 {
            msg!("StateMerkleTreeAccount data too short for discriminator");
            return Err(LightSdkError::from(ProgramError::InvalidAccountData));
        }

        let discriminator = &account_data[0..8];
        if discriminator != STATE_MERKLE_TREE_DISCRIMINATOR {
            msg!("Invalid StateMerkleTreeAccount discriminator");
            return Err(LightSdkError::from(ProgramError::InvalidAccountData));
        }

        let merkle_tree = state_merkle_tree_from_bytes_zero_copy(&account_data)
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;

        Ok(merkle_tree.roots[root_index as usize])
    }
}
