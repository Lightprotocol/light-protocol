use anchor_lang::solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey,
};
use spl_token::instruction::TokenInstruction;

#[repr(u8)]
pub enum InstructionType {
    DecompressedTransfer = 3,
    Other,
}

impl From<u8> for InstructionType {
    fn from(value: u8) -> Self {
        match value {
            3 => InstructionType::DecompressedTransfer,
            _ => InstructionType::Other,
        }
    }
}

#[cfg(feature = "cpi")]
anchor_lang::solana_program::entrypoint!(process_instruction);

pub fn process_instruction<'a, 'b, 'c, 'info>(
    program_id: &'a Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &'c [u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::from(instruction_data[0]);
    match discriminator {
        // TODO: match anchor instructions before
        InstructionType::DecompressedTransfer => {
            // unpack instruction
            let instruction = TokenInstruction::unpack(instruction_data)?;
            match instruction {
                TokenInstruction::Transfer { amount } => {
                    spl_token::processor::Processor::process_transfer(
                        program_id, accounts, amount, None, // TODO: check where to get these
                    )
                }
                _ => Err(ProgramError::InvalidInstructionData),
            }
        }
        // InstructionType::UpdatePdaBorsh => {
        //     update_pda::update_pda::<false>(accounts, &instruction_data[1..])
        // }
        _ => light_compressed_token::entry(program_id, accounts, instruction_data),
    }?;
    Ok(())
}
