use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::{errors::AccountCompressionErrorCode, utils::constants::NOOP_PUBKEY};

#[inline(never)]
pub fn emit_indexer_event(data: Vec<u8>, noop_program: &AccountInfo) -> Result<()> {
    if noop_program.key() != Pubkey::new_from_array(NOOP_PUBKEY) || !noop_program.executable {
        return err!(AccountCompressionErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}
