use anchor_lang::prelude::*;

use crate::PROTOCOL_CONFIG_PDA_SEED;

use super::state::ProtocolConfigPda;

#[derive(Accounts)]
pub struct UpdateProtocolConfig<'info> {
    /// CHECK: authority is protocol config authority.
    #[account(mut, constraint = authority.key() == protocol_config_pda.authority)]
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints).
    #[account(mut, seeds = [PROTOCOL_CONFIG_PDA_SEED], bump)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: is signer to reduce risk of updating with a wrong authority.
    pub new_authority: Signer<'info>,
}
