use light_sdk_pinocchio::error::LightSdkError;
use pinocchio::{
    account_info::AccountInfo, entrypoint, msg, program_error::ProgramError, pubkey::Pubkey,
};

pub mod create_pda;
pub mod update_pda;

pub const ID: Pubkey = [
    135, 152, 63, 145, 194, 241, 126, 41, 180, 254, 157, 105, 170, 129, 15, 255, 138, 167, 39, 151,
    70, 146, 233, 196, 238, 88, 139, 37, 169, 154, 138, 188,
];

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
    msg!(format!("instruction_data: {:?}", instruction_data[..8].to_vec()).as_str());
    let discriminator = InstructionType::try_from(instruction_data[0]).unwrap();
    msg!(format!("instruction_data: {:?}", instruction_data[..8].to_vec()).as_str());
    match discriminator {
        InstructionType::CreatePdaBorsh => {
            create_pda::create_pda::<true>(accounts, &instruction_data[1..])
        }
        InstructionType::UpdatePdaBorsh => {
            update_pda::update_pda::<true>(accounts, &instruction_data[1..])
        }
    }?;
    Ok(())
}
