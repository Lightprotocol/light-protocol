use anchor_lang::{
    solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey},
    Discriminator,
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

#[cfg(not(feature = "cpi"))]
anchor_lang::solana_program::entrypoint!(process_instruction);

pub fn process_instruction<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::from(instruction_data[0]);
    match discriminator {
        InstructionType::DecompressedTransfer => {
            let instruction = TokenInstruction::unpack(instruction_data)?;
            match instruction {
                TokenInstruction::Transfer { amount } => {
                    spl_token::processor::Processor::process_transfer(
                        program_id, accounts, amount, None,
                    )?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
        // anchor instructions have no discriminator conflicts with InstructionType
        _ => light_compressed_token::entry(program_id, accounts, instruction_data)?,
    }

    //     light_compressed_token::instruction::CreateCompressedMint::DISCRIMINATOR
    //     | light_compressed_token::instruction::MintToCompressed::DISCRIMINATOR
    //     | light_compressed_token::instruction::CreateSplMint::DISCRIMINATOR
    //     | light_compressed_token::instruction::CreateTokenPool::DISCRIMINATOR
    //     | light_compressed_token::instruction::AddTokenPool::DISCRIMINATOR
    //     | light_compressed_token::instruction::MintTo::DISCRIMINATOR
    //     | light_compressed_token::instruction::BatchCompress::DISCRIMINATOR
    //     | light_compressed_token::instruction::CompressSplTokenAccount::DISCRIMINATOR
    //     | light_compressed_token::instruction::Transfer::DISCRIMINATOR
    //     | light_compressed_token::instruction::Approve::DISCRIMINATOR
    //     | light_compressed_token::instruction::Revoke::DISCRIMINATOR
    //     | light_compressed_token::instruction::Freeze::DISCRIMINATOR
    //     | light_compressed_token::instruction::Thaw::DISCRIMINATOR
    //     | light_compressed_token::instruction::Burn::DISCRIMINATOR => {
    //         light_compressed_token::entry(program_id, accounts, instruction_data)?;

    Ok(())
}
