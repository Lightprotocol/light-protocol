use crate::protocol_config::state::ProtocolConfig;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::protocol_config::state::ProtocolConfigPda;
use aligned_sized::aligned_sized;

#[aligned_sized(anchor)]
#[account]
#[derive(Debug, Default, Copy, PartialEq, Eq)]
pub struct ForesterAccount {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub active_stake_weight: u64,
    /// Pending stake which will get active once the next epoch starts.
    pub pending_undelegated_stake_weight: u64,
    pub current_epoch: u64,
    /// Link to previous compressed forester epoch account hash.
    pub last_compressed_forester_epoch_pda_hash: [u8; 32],
    pub last_registered_epoch: u64,
    pub last_claimed_epoch: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct ForesterConfig {
    /// Fee in percentage points.
    pub fee: u64,
    pub fee_recipient: Pubkey,
}

impl ForesterAccount {
    /// If current epoch changed, move pending stake to active stake and update
    /// current epoch field
    pub fn sync(&mut self, current_slot: u64, protocol_config: &ProtocolConfig) -> Result<()> {
        // get last registration epoch, stake sync treats the registration phase
        // of an epoch like the next active epoch
        let current_epoch = protocol_config.get_current_epoch(
            current_slot.saturating_sub(protocol_config.registration_phase_length),
        );
        // msg!("current_epoch: {}", current_epoch);
        // msg!("self.current_epoch: {}", self.current_epoch);
        // If the current epoch is greater than the last registered epoch, or next epoch is in registration phase
        if current_epoch > self.current_epoch
            || protocol_config.is_registration_phase(current_slot).is_ok()
        {
            // msg!("self pending stake weight: {}", self.pending_undelegated_stake_weight);
            // msg!("self active stake weight: {}", self.active_stake_weight);
            self.current_epoch = current_epoch;
            self.active_stake_weight += self.pending_undelegated_stake_weight;
            self.pending_undelegated_stake_weight = 0;
            // msg!("self pending stake weight: {}", self.pending_undelegated_stake_weight);
            // msg!("self active stake weight: {}", self.active_stake_weight);
        }
        Ok(())
    }
}

pub const FORESTER_SEED: &[u8] = b"forester";
pub const FORESTER_TOKEN_POOL_SEED: &[u8] = b"forester_token_pool";
#[derive(Accounts)]
pub struct RegisterForester<'info> {
    /// CHECK:
    #[account(init, seeds = [FORESTER_SEED, authority.key().to_bytes().as_slice()], bump, space =ForesterAccount::LEN , payer = signer)]
    pub forester_pda: Account<'info, ForesterAccount>,
    #[account(
        init,
        seeds = [
        FORESTER_TOKEN_POOL_SEED, authority.key().to_bytes().as_slice(),
        ],
        bump,
        payer = signer,
          token::mint = mint,
          token::authority = cpi_authority_pda,
    )]
    pub token_pool_pda: Account<'info, TokenAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    #[account(constraint = mint.key() == protocol_config_pda.config.mint)]
    pub mint: Account<'info, Mint>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateForester<'info> {
    /// CHECK:
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterAccount>,
    pub authority: Signer<'info>,
    pub new_authority: Option<Signer<'info>>,
}
