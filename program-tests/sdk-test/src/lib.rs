use light_macros::pubkey;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer, error::LightSdkError};
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
};

pub mod compress_dynamic_pda;
pub mod create_config;
pub mod create_dynamic_pda;
pub mod create_pda;
pub mod decompress_dynamic_pda;
pub mod update_config;
pub mod update_pda;

pub const ID: Pubkey = pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    UpdatePdaBorsh = 1,
    CompressDynamicPda = 2,
    CreateDynamicPda = 3,
    InitializeCompressionConfig = 4,
    UpdateCompressionConfig = 5,
    DecompressMultipleAccountsIdempotent = 6,
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            1 => Ok(InstructionType::UpdatePdaBorsh),
            2 => Ok(InstructionType::CompressDynamicPda),
            3 => Ok(InstructionType::CreateDynamicPda),
            4 => Ok(InstructionType::InitializeCompressionConfig),
            5 => Ok(InstructionType::UpdateCompressionConfig),
            6 => Ok(InstructionType::DecompressMultipleAccountsIdempotent),

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
        InstructionType::CompressDynamicPda => {
            compress_dynamic_pda::compress_dynamic_pda(accounts, &instruction_data[1..])
        }
        InstructionType::CreateDynamicPda => {
            create_dynamic_pda::create_dynamic_pda(accounts, &instruction_data[1..])
        }

        InstructionType::InitializeCompressionConfig => {
            create_config::process_initialize_compression_config_checked(
                accounts,
                &instruction_data[1..],
            )
        }
        InstructionType::UpdateCompressionConfig => {
            update_config::process_update_config(accounts, &instruction_data[1..])
        }
        InstructionType::DecompressMultipleAccountsIdempotent => {
            decompress_dynamic_pda::decompress_multiple_dynamic_pdas(
                accounts,
                &instruction_data[1..],
            )
        }
    }?;
    Ok(())
}
