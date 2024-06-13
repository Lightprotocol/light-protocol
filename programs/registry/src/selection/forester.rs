use crate::protocol_config::state::ProtocolConfig;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

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
    pub pending_stake_weight: u64,
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

impl ForesterAccount {
    /// If current epoch changed, move pending stake to active stake and update
    /// current epoch field
    pub fn sync(&mut self, current_slot: u64, protocol_config: &ProtocolConfig) -> Result<()> {
        // get last registration epoch, stake sync treats the registration phase
        // of an epoch like the next active epoch
        let current_epoch = protocol_config.get_current_epoch(
            current_slot.saturating_sub(protocol_config.registration_phase_length),
        );
        // If the current epoch is greater than the last registered epoch, or next epoch is in registration phase
        if current_epoch > self.current_epoch
            || protocol_config.is_registration_phase(current_slot).is_ok()
        {
            self.current_epoch = current_epoch;
            self.active_stake_weight += self.pending_stake_weight;
            self.pending_stake_weight = 0;
        }
        Ok(())
    }
}

pub const FORESTER_SEED: &[u8] = b"forester";

#[derive(Accounts)]
#[instruction(bump: u8, authority: Pubkey)]
pub struct RegisterForester<'info> {
    /// CHECK:
    #[account(init, seeds = [FORESTER_SEED, authority.to_bytes().as_slice()], bump, space =ForesterAccount::LEN , payer = signer)]
    pub forester_pda: Account<'info, ForesterAccount>,
    #[account(mut, address = authority_pda.authority)]
    pub signer: Signer<'info>,
    pub authority_pda: Account<'info, ProtocolConfigPda>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateForester<'info> {
    /// CHECK:
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterAccount>,
    pub authority: Signer<'info>,
    pub new_authority: Option<Signer<'info>>,
}
