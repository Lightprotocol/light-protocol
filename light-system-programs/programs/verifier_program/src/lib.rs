use anchor_lang::prelude::*;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
    use super::*;

    /// Saves the provided message in a temporary PDA.
    pub fn shielded_transfer_first<'info>(
        ctx: Context<LightInstructionFirst<'info>>,
        msg: Vec<u8>,
    ) -> Result<()> {
        let state = &mut ctx.accounts.verifier_state;
        state.msg = msg.clone();

        Ok(())
    }

    /// Close the temporary PDA. Should be used when we don't intend to perform
    /// the second transfer and want to reclaim the funds.
    pub fn shielded_transfer_close<'info>(
        _ctx: Context<LightInstructionClose<'info>>,
    ) -> Result<()> {
        Ok(())
    }
}

#[account]
pub struct VerifierState {
    pub msg: Vec<u8>,
}

#[derive(Accounts)]
pub struct LightInstructionFirst<'info> {
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED],
        bump,
        space = 1024 + 8,
        payer = signing_address
    )]
    pub verifier_state: Account<'info, VerifierState>,
}

#[derive(Accounts)]
pub struct LightInstructionClose<'info> {
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut, close=signing_address)]
    pub verifier_state: Account<'info, VerifierState>,
}
