use crate::utils::{accounts::create_and_check_pda, constants::NULLIFIER_SEED};
use crate::RegisteredVerifier;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, sysvar};

#[derive(Accounts)]
pub struct InitializeNullifiers<'info> {
    /// CHECK:` Signer is owned by registered verifier program.
    #[account(mut, seeds=[__program_id.to_bytes().as_ref()], bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump )]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
}

/// Inserts nullifiers, written in plain rust for memory optimization.
pub fn process_insert_nullifiers<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeNullifiers<'info>>,
    nullifiers: Vec<[u8; 32]>,
) -> Result<()> {
    let rent = <Rent as sysvar::Sysvar>::get()?;

    for (nullifier_pda, nullifier) in ctx.remaining_accounts.iter().zip(nullifiers) {
        create_and_check_pda(
            ctx.program_id,
            &ctx.accounts.authority.to_account_info(),
            &nullifier_pda.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
            &nullifier,
            NULLIFIER_SEED,
            1,    //bytes
            0,    //lamports
            true, //rent_exempt
        )
        .unwrap();
        nullifier_pda.to_account_info().data.borrow_mut()[0] = 1u8;
    }
    Ok(())
}
