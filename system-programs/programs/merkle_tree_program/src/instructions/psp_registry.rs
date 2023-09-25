use anchor_lang::prelude::*;

#[account]
pub struct RegisteredPsp {
    pub solana_pubkey: [u8; 32],
    pub index: u64,
}

#[derive(Accounts)]
#[instruction(index: u64)]
pub struct RegisterPsp<'info> {
    #[account(
        init,
        space = 8 + 32,
        seeds = [REGISTERED_PSP_SEED, signer.key().to_bytes().as_ref()],
        bump,
        payer = signer,
    )]
    pub registered_psp: Account<'info, RegisteredPsp>,
    #[accounts(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
