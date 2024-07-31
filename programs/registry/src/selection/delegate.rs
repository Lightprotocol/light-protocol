use super::forester::ForesterAccount;
use crate::protocol_config::state::ProtocolConfig;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DelegateAccount {
    pub delegate_forester_delegate_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    pub delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    pub stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pub pending_stake_weight: u64,
    pub pending_epoch: u64,
    pub last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pub pending_token_amount: u64,
}

impl DelegateAccount {
    pub fn sync_pending_stake_weight(&mut self, current_epoch: u64) {
        if current_epoch > self.last_sync_epoch {
            self.stake_weight += self.pending_stake_weight;
            self.pending_stake_weight = 0;
            self.pending_epoch = current_epoch;
        }
    }
}

// // TODO: create delegate account instruction
// pub fn deposit_delegate_account_instruction(
//     delegate_account: &mut DelegateAccount,
//     num_tokens: u64,
//     token_account: &mut TokenAccount,
//     delegate_token_account: &mut TokenAccount,
// ) -> Result<()> {
//     delegate_account.stake_weight += num_tokens;

//     // TODO: add token transfer
//     // delegate_token_account.balance += num_tokens;
//     // token_account.balance -= num_tokens;
//     Ok(())
// }

pub fn delegate_instruction(
    protocol_config: &ProtocolConfig,
    delegate_account: &mut DelegateAccount,
    forester_delegate_account_pubkey: &Pubkey,
    forester_pda: &mut ForesterAccount,
    num_tokens: u64,
    current_slot: u64,
    no_sync: bool,
) -> Result<()> {
    if !no_sync {
        forester_pda.sync(current_slot, protocol_config)?;
    }
    // TODO: check that is not delegated to a different forester
    delegate_account.delegate_forester_delegate_account = Some(*forester_delegate_account_pubkey);
    forester_pda.pending_stake_weight += num_tokens;
    delegate_account.delegated_stake_weight += num_tokens;
    delegate_account.stake_weight -= num_tokens;
    Ok(())
}

pub fn undelegate_instruction(
    protocol_config: &ProtocolConfig,
    delegate_account: &mut DelegateAccount,
    forester_pda: &mut ForesterAccount,
    amount: u64,
    current_slot: u64,
) -> Result<()> {
    forester_pda.sync(current_slot, protocol_config)?;
    forester_pda.active_stake_weight -= amount;
    delegate_account.delegated_stake_weight -= amount;
    delegate_account.pending_stake_weight += amount;
    delegate_account.pending_epoch = protocol_config.get_current_epoch(current_slot);
    if delegate_account.delegated_stake_weight == 0 {
        delegate_account.delegate_forester_delegate_account = None;
    }
    Ok(())
}

// TODO: we need a drastically improved compressed token transfer sdk
// pub fn withdraw_instruction(
//     delegate_account: &mut DelegateAccount,
//     delegate_token_account: &mut AccountInfo,
//     recipient_token_account: &mut AccountInfo,
//     protocol_config: ProtocolConfig,
//     amount: u64,
//     current_slot: u64,
// ) -> Result<()> {
//     withdraw(delegate_account, protocol_config, amount, current_slot);
//     // transfer tokens
//     // TODO: add compressed token transfer
//     // delegate_token_account.balance -= amount;
//     // recipient_token_account.balance += amount;
//     Ok(())
// }

#[allow(unused)]
fn withdraw(
    delegate_account: &mut DelegateAccount,
    protocol_config: ProtocolConfig,
    amount: u64,
    current_slot: u64,
) {
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    delegate_account.sync_pending_stake_weight(current_epoch);
    // reduce stake weight
    // only non delegated stake can be unstaked
    delegate_account.stake_weight -= amount;
}
