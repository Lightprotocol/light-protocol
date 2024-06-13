use crate::protocol_config::state::ProtocolConfig;
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::{errors::HasherError, DataHasher, Hasher};
use light_utils::hash_to_bn254_field_size_be;

/// Instruction data input verion of DelegateAccount The following fields are
/// missing since these are computed onchain:
/// 1. owner
/// 2. escrow_token_account_hash
/// -> we save 64 bytes in instructiond data
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct InputDelegateAccount {
    pub delegate_forester_delegate_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    pub delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    pub stake_weight: u64,
    /// When delegating stake is pending until the next epoch
    pub pending_delegated_stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pub pending_undelegated_stake_weight: u64,
    pub pending_synced_stake_weight: u64,
    pub pending_epoch: u64,
    pub last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pub pending_token_amount: u64,
}

impl From<DelegateAccount> for InputDelegateAccount {
    fn from(delegate_account: DelegateAccount) -> Self {
        InputDelegateAccount {
            delegate_forester_delegate_account: delegate_account.delegate_forester_delegate_account,
            delegated_stake_weight: delegate_account.delegated_stake_weight,
            stake_weight: delegate_account.stake_weight,
            pending_undelegated_stake_weight: delegate_account.pending_undelegated_stake_weight,
            pending_epoch: delegate_account.pending_epoch,
            last_sync_epoch: delegate_account.last_sync_epoch,
            pending_token_amount: delegate_account.pending_token_amount,
            pending_synced_stake_weight: delegate_account.pending_synced_stake_weight,
            pending_delegated_stake_weight: delegate_account.pending_delegated_stake_weight,
        }
    }
}

#[aligned_sized]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct DelegateAccount {
    pub owner: Pubkey,
    pub delegate_forester_delegate_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    pub delegated_stake_weight: u64,
    /// newly delegated stakeweight becomes active after the next epoch
    pub pending_delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    pub stake_weight: u64,
    /// Buffer variable to account for the lag of one epoch for rewards to reach
    /// to registration account
    pub pending_synced_stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pub pending_undelegated_stake_weight: u64,
    pub pending_epoch: u64,
    pub last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pub pending_token_amount: u64,
    pub escrow_token_account_hash: [u8; 32],
}

pub trait CompressedAccountTrait {
    fn get_owner(&self) -> Pubkey;
}
impl CompressedAccountTrait for DelegateAccount {
    fn get_owner(&self) -> Pubkey {
        self.owner
    }
}

// TODO: pass in hashed owner
impl DataHasher for DelegateAccount {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_owner = hash_to_bn254_field_size_be(self.owner.as_ref()).unwrap().0;
        let hashed_delegate_forester_delegate_account =
            if let Some(delegate_forester_delegate_account) =
                self.delegate_forester_delegate_account
            {
                hash_to_bn254_field_size_be(delegate_forester_delegate_account.as_ref())
                    .unwrap()
                    .0
            } else {
                [0u8; 32]
            };
        H::hashv(&[
            hashed_owner.as_slice(),
            hashed_delegate_forester_delegate_account.as_slice(),
            &self.delegated_stake_weight.to_le_bytes(),
            &self.pending_synced_stake_weight.to_le_bytes(),
            &self.stake_weight.to_le_bytes(),
            &self.pending_undelegated_stake_weight.to_le_bytes(),
        ])
    }
}

impl DelegateAccount {
    pub fn sync_pending_stake_weight(&mut self, current_epoch: u64) {
        msg!("sync_pending_stake_weight current_epoch: {}", current_epoch);
        msg!(
            "sync_pending_stake_weight pending_epoch: {}",
            self.pending_epoch
        );
        if current_epoch > self.pending_epoch {
            self.stake_weight += self.pending_undelegated_stake_weight;
            self.pending_undelegated_stake_weight = 0;
            // last sync epoch is only relevant for syncing the delegate account with the forester rewards
            // self.last_sync_epoch = current_epoch;
            self.delegated_stake_weight += self.pending_delegated_stake_weight;
            self.pending_delegated_stake_weight = 0;
            // self.pending_epoch = 0;
        }
    }
}

// pub fn undelegate(
//     protocol_config: &ProtocolConfig,
//     delegate_account: &mut DelegateAccount,
//     forester_pda: &mut ForesterAccount,
//     amount: u64,
//     current_slot: u64,
// ) -> Result<()> {
//     forester_pda.sync(current_slot, protocol_config)?;
//     forester_pda.active_stake_weight -= amount;
//     delegate_account.delegated_stake_weight -= amount;
//     delegate_account.pending_undelegated_stake_weight += amount;
//     delegate_account.pending_epoch = protocol_config.get_current_epoch(current_slot);
//     if delegate_account.delegated_stake_weight == 0 {
//         delegate_account.delegate_forester_delegate_account = None;
//     }
//     Ok(())
// }

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
/**
 * User flow:
 * 1. Deposit compressed tokens to DelegatePda
 * - inputs: InputTokenData, deposit_amount
 * - create two outputs, escrow compressed account and change account
 * - compressed escrow account is owned by pda derived from authority
 * -
 */
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
