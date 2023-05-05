use std::str::FromStr;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

pub const NOOP_PROGRAM_ID: &str = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";

#[error_code]
pub enum AccountCompressionError {
    #[msg("The provided program is not the noop program.")]
    NoopProgram,
}

/// Sends the given data to the noop program.
pub fn wrap_event<'info>(
    data: &[u8],
    noop_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
) -> Result<()> {
    if noop_program.key() != Pubkey::from_str(NOOP_PROGRAM_ID).unwrap() {
        return Err(AccountCompressionError::NoopProgram.into());
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data: data.to_vec(),
    };
    invoke(
        &instruction,
        &[noop_program.to_account_info(), signer.to_account_info()],
    )?;
    Ok(())
}
