use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct GenericInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
}
