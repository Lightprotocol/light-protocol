#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
mod test_4rkyv;
use test_4rkyv::{test_rkyv, test_rkyv_zero_copy};
declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[derive(Accounts)]
pub struct RkyvTest<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[program]
pub mod test_rkyv {

    use super::*;

    pub fn invoke_test_rkyv<'info>(
        _ctx: Context<'_, '_, '_, 'info, RkyvTest<'info>>,
    ) -> Result<()> {
        test_rkyv();
        // test_rkyv_zero_copy();
        Ok(())
    }
}
