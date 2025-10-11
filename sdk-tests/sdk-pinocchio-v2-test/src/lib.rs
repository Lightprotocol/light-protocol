use light_macros::pubkey_array;
use light_sdk_pinocchio::{derive_light_cpi_signer, error::LightSdkError, CpiSigner};
use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
};
pub mod create_pda;
pub mod update_pda;

pub const ID: Pubkey = pubkey_array!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    UpdatePdaBorsh = 1,
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            1 => Ok(InstructionType::UpdatePdaBorsh),
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
        InstructionType::CreatePdaBorsh => create_pda::create_pda(accounts, &instruction_data[1..]),
        InstructionType::UpdatePdaBorsh => {
            update_pda::update_pda::<true>(accounts, &instruction_data[1..])
        }
    }?;
    Ok(())
}
