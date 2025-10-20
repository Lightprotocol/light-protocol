use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;

pub mod v1 {
    use light_account_checks::checks::check_owner;
    use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopy;
    use light_hasher::Poseidon;
    use light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID;

    use super::*;

    /// StateMerkleTreeAccount discriminator
    pub const STATE_MERKLE_TREE_DISCRIMINATOR: [u8; 8] = [172, 43, 172, 186, 29, 73, 219, 84];
    pub const STATE_MERKLE_TREE_ACCOUNT_METADATA_LEN: usize = 224;

    /// Reads a root from the concurrent state merkle tree by index
    pub fn read_state_merkle_tree_root(
        account_info: &AccountInfo,
        root_index: u16,
    ) -> Result<[u8; 32], ProgramError> {
        if root_index as usize >= 2400 {
            msg!(
                "Invalid root index: {} greater than max root index {}",
                root_index,
                2400
            );
            return Err(ProgramError::InvalidArgument);
        }
        check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_info)?;
        let account_data = account_info.try_borrow_data()?;

        // Check discriminator
        if account_data.len() < 8 {
            msg!("StateMerkleTreeAccount data too short for discriminator");
            return Err(ProgramError::InvalidAccountData);
        }

        let discriminator = &account_data[0..8];
        if discriminator != STATE_MERKLE_TREE_DISCRIMINATOR {
            msg!("Invalid StateMerkleTreeAccount discriminator");
            return Err(ProgramError::InvalidAccountData);
        }
        let required_size = STATE_MERKLE_TREE_ACCOUNT_METADATA_LEN;
        if account_data.len() < required_size {
            msg!("StateMerkleTreeAccount data too short for metadata");
            return Err(ProgramError::InvalidAccountData);
        }

        let data = &account_data[required_size..];
        let merkle_tree = ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(data)?;

        Ok(merkle_tree.roots[root_index as usize])
    }
}
