use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use light_macros::pubkey;

use crate::errors::AccountCompressionErrorCode;

pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

#[inline(never)]
pub fn emit_indexer_event<'info>(
    data: Vec<u8>,
    noop_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
) -> Result<()> {
    if noop_program.key() != NOOP_PROGRAM_ID {
        return err!(AccountCompressionErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data,
    };
    invoke(
        &instruction,
        &[noop_program.to_account_info(), signer.to_account_info()],
    )?;
    Ok(())
}
