use crate::{
    constants::{FORESTER_SEED, FORESTER_TOKEN_POOL_SEED},
    protocol_config::state::ProtocolConfig,
};
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
    // TODO: switch to promille
    /// Fee in percentage points.
    pub fee: u64,
    pub fee_recipient: Pubkey,
}

impl ForesterAccount {
    ///  Sync should be called in any instruction which uses the forester
    /// account before any action. Delegating to a forester new stakeweight is
    /// added to the pending stake. If current epoch changed, move pending stake
    /// to active stake and update current epoch field. This method is called in
    /// delegate stake so that it is impossible to delegate and not sync before.
    pub fn sync(&mut self, current_slot: u64, protocol_config: &ProtocolConfig) -> Result<()> {
        // get last registration epoch, stake sync treats the registration phase
        // of an epoch like the next active epoch
        let current_epoch = protocol_config.get_current_registration_epoch(current_slot);
        if current_epoch > self.current_epoch {
            self.current_epoch = current_epoch;
            self.active_stake_weight += self.pending_undelegated_stake_weight;
            self.pending_undelegated_stake_weight = 0;
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RegisterForester<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    #[account(constraint = mint.key() == protocol_config_pda.config.mint)]
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(init, seeds = [FORESTER_SEED, authority.key().as_ref()], bump, space =ForesterAccount::LEN , payer = fee_payer)]
    pub forester_pda: Account<'info, ForesterAccount>,
    #[account(
        init,
        seeds = [
        FORESTER_TOKEN_POOL_SEED, forester_pda.key().as_ref(),
        ],
        bump,
        payer = fee_payer,
          token::mint = mint,
          token::authority = cpi_authority_pda,
    )]
    pub token_pool_pda: Account<'info, TokenAccount>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateForester<'info> {
    pub authority: Signer<'info>,
    /// CHECK: authority is forester pda authority.
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterAccount>,
    pub new_authority: Option<Signer<'info>>,
}
