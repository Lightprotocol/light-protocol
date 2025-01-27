use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct GenericInstruction<'info> {
    pub authority: Signer<'info>,
}
