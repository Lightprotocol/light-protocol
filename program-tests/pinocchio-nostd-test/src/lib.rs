#![no_std]
#![deny(warnings)]
#![cfg_attr(target_os = "solana", forbid(unsafe_code))]
// Ensure we're truly no_std by forbidding these
#![cfg_attr(not(test), no_main)]

use light_macros::pubkey_array;
use light_sdk_pinocchio::{derive_light_cpi_signer, error::LightSdkError, CpiSigner};
use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
};

pub const ID: Pubkey = pubkey_array!("NoStDPinocchio11111111111111111111111111111");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("NoStDPinocchio11111111111111111111111111111");

// Use the pinocchio entrypoint! macro which sets up:
// - Program entrypoint
// - Default bump allocator
// - Default panic handler
entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    TestBasic = 0,
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::TestBasic),
            _ => panic!("Invalid instruction discriminator."),
        }
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::try_from(instruction_data[0])?;
    match discriminator {
        InstructionType::TestBasic => {
            // Basic test - just verify we can execute in no_std environment
            Ok(())
        }
    }
}

// Compile-time assertion that we're truly no_std
// This will fail to compile if std is somehow available
#[cfg(all(target_os = "solana", feature = "std"))]
compile_error!("std feature must not be enabled for Solana target!");

// Verify that the std crate is not available
#[cfg(target_os = "solana")]
const _: () = {
    // This would fail to compile if std was available
    // because we've declared #![no_std] at the crate level
    #[cfg(feature = "std")]
    compile_error!("ERROR: std is available in a no_std crate!");
};

#[cfg(not(target_os = "solana"))]
pub mod test_helpers {
    use super::*;

    pub fn get_program_id() -> Pubkey {
        ID
    }

    pub fn get_cpi_signer() -> CpiSigner {
        LIGHT_CPI_SIGNER
    }
}
