use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{constants::FORESTER_SEED, protocol_config::state::ProtocolConfigPda};

#[aligned_sized(anchor)]
#[account]
#[derive(Debug, Default, Copy, PartialEq, Eq)]
pub struct ForesterPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub active_weight: u64,
    /// Pending weight which will get active once the next epoch starts.
    pub pending_weight: u64,
    pub current_epoch: u64,
    /// Link to previous compressed forester epoch account hash.
    pub last_compressed_forester_epoch_pda_hash: [u8; 32],
    pub last_registered_epoch: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct ForesterConfig {
    /// Fee in percentage points.
    pub fee: u64,
}

#[derive(Accounts)]
#[instruction(bump: u8, forester_authority: Pubkey)]
pub struct RegisterForester<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(has_one = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    #[account(init, seeds = [FORESTER_SEED, forester_authority.as_ref()], bump, space =ForesterPda::LEN , payer = fee_payer)]
    pub forester_pda: Account<'info, ForesterPda>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateForesterPda<'info> {
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterPda>,
    pub new_authority: Option<Signer<'info>>,
}

#[derive(Accounts)]
pub struct UpdateForesterPdaWeight<'info> {
    pub authority: Signer<'info>,
    #[account(has_one = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    #[account(mut)]
    pub forester_pda: Account<'info, ForesterPda>,
}
