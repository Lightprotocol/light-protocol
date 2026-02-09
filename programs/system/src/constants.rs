// Re-export shared constants from light-vm
pub use light_vm::constants::{
    CPI_AUTHORITY_PDA_BUMP, CPI_AUTHORITY_PDA_SEED, CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR,
    CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR, MAX_OUTPUT_ACCOUNTS,
};

use crate::{errors::SystemProgramError, InstructionDiscriminator};

pub const INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];
pub const INVOKE_INSTRUCTION: [u8; 8] = [26, 16, 169, 7, 21, 202, 242, 25];
pub const INVOKE_CPI_INSTRUCTION: [u8; 8] = [49, 212, 191, 129, 39, 194, 43, 196];
pub const INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION: [u8; 8] = [86, 47, 163, 166, 21, 223, 92, 8];
pub const INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION: [u8; 8] = [228, 34, 128, 84, 47, 139, 86, 240];
pub const RE_INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION: [u8; 8] =
    [187, 147, 22, 142, 104, 180, 136, 190];

impl TryFrom<&[u8]> for InstructionDiscriminator {
    type Error = crate::errors::SystemProgramError;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let array: [u8; 8] = value
            .try_into()
            .map_err(|_| crate::errors::SystemProgramError::InvalidArgument)?;
        match array {
            INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION => {
                Ok(InstructionDiscriminator::InitializeCpiContextAccount)
            }
            INVOKE_INSTRUCTION => Ok(InstructionDiscriminator::Invoke),
            INVOKE_CPI_INSTRUCTION => Ok(InstructionDiscriminator::InvokeCpi),
            INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION => {
                Ok(InstructionDiscriminator::InvokeCpiWithReadOnly)
            }
            INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION => {
                Ok(InstructionDiscriminator::InvokeCpiWithAccountInfo)
            }
            #[cfg(feature = "reinit")]
            RE_INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION => {
                Ok(InstructionDiscriminator::ReInitCpiContextAccount)
            }
            _ => Err(SystemProgramError::InvalidInstructionDataDiscriminator),
        }
    }
}

#[cfg(test)]
mod test {
    use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
    use solana_pubkey::Pubkey;

    use super::*;
    use crate::processor::sol_compression::{SOL_POOL_PDA_BUMP, SOL_POOL_PDA_SEED};

    fn check_hardcoded_bump(program_id: Pubkey, seeds: &[&[u8]], bump: u8) -> bool {
        let (_, found_bump) = Pubkey::find_program_address(seeds, &program_id);
        found_bump == bump
    }

    #[test]
    fn test_account_compression_cpi_authority_bump() {
        assert!(check_hardcoded_bump(
            ACCOUNT_COMPRESSION_PROGRAM_ID.into(),
            &[CPI_AUTHORITY_PDA_SEED],
            CPI_AUTHORITY_PDA_BUMP
        ));
    }

    #[test]
    fn test_check_anchor_option_sol_pool_pda() {
        assert!(check_hardcoded_bump(
            crate::ID.into(),
            &[SOL_POOL_PDA_SEED],
            SOL_POOL_PDA_BUMP
        ));
    }
}
