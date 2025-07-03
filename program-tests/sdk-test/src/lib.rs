use light_macros::pubkey;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer, error::LightSdkError};
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
};

pub mod compress_from_pda;
pub mod create_pda;
pub mod decompress_to_pda;
pub mod update_decompressed_pda;
pub mod update_pda;

pub const ID: Pubkey = pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    UpdatePdaBorsh = 1,
    DecompressToPda = 2,
    CompressFromPda = 3,
    UpdateDecompressedPda = 4,
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            1 => Ok(InstructionType::UpdatePdaBorsh),
            2 => Ok(InstructionType::DecompressToPda),
            3 => Ok(InstructionType::CompressFromPda),
            4 => Ok(InstructionType::UpdateDecompressedPda),
            _ => panic!("Invalid instruction discriminator."),
        }
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::try_from(instruction_data[0]).unwrap();
    match discriminator {
        InstructionType::CreatePdaBorsh => {
            create_pda::create_pda::<true>(accounts, &instruction_data[1..])
        }
        InstructionType::UpdatePdaBorsh => {
            update_pda::update_pda::<false>(accounts, &instruction_data[1..])
        }
        InstructionType::DecompressToPda => {
            decompress_to_pda::decompress_to_pda(accounts, &instruction_data[1..])
        }
        InstructionType::CompressFromPda => {
            compress_from_pda::compress_from_pda(accounts, &instruction_data[1..])
        }
        InstructionType::UpdateDecompressedPda => {
            update_decompressed_pda::update_decompressed_pda(accounts, &instruction_data[1..])
        }
    }?;
    Ok(())
}
