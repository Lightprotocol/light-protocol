use anchor_lang::prelude::*;

use crate::AUTHORITY_PDA_SEED;

use super::state::ProtocolConfigPda;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct UpdateAuthority<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    pub authority: Signer<'info>,
    /// CHECK:
    // TODO: rename to protocol config pda
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump)]
    pub authority_pda: Account<'info, ProtocolConfigPda>,
    pub new_authority: Signer<'info>,
}
