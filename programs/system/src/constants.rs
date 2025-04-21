use crate::{errors::SystemProgramError, InstructionDiscriminator};

pub const CPI_AUTHORITY_PDA_BUMP: u8 = 255;

pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

pub const MAX_OUTPUT_ACCOUNTS: usize = 30;

pub const INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];
pub const INVOKE_INSTRUCTION: [u8; 8] = [26, 16, 169, 7, 21, 202, 242, 25];
pub const INVOKE_CPI_INSTRUCTION: [u8; 8] = [49, 212, 191, 129, 39, 194, 43, 196];
pub const INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION: [u8; 8] = [86, 47, 163, 166, 21, 223, 92, 8];
pub const CPI_CONTEXT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];
pub const INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION: [u8; 8] = [228, 34, 128, 84, 47, 139, 86, 240];

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
            _ => Err(SystemProgramError::InvalidInstructionDataDiscriminator),
        }
    }
}
