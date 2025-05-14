#![allow(unexpected_cfgs)]
pub mod batch;
pub mod constants;
pub mod errors;
pub mod initialize_address_tree;
pub mod initialize_state_tree;
pub mod merkle_tree;
pub mod merkle_tree_metadata;
pub mod queue;
pub mod queue_batch_metadata;
pub mod rollover_address_tree;
pub mod rollover_state_tree;

// Use the appropriate BorshDeserialize and BorshSerialize based on feature
use borsh::{BorshDeserialize, BorshSerialize};
// Pinocchio imports when pinocchio feature is enabled
#[cfg(feature = "pinocchio")]
use pinocchio::{
    account_info::AccountInfo, msg, pubkey::Pubkey, sysvars::rent::Rent, sysvars::Sysvar,
};
// Solana program imports for non-pinocchio builds (default)
#[cfg(not(feature = "pinocchio"))]
pub(crate) use {
    solana_account_info::AccountInfo,
    solana_msg::msg,
    solana_pubkey::Pubkey,
    solana_sysvar::{rent::Rent, Sysvar},
};

#[allow(unused)]
trait AccountInfoTrait {
    fn key(&self) -> &Pubkey;
}

#[cfg(not(feature = "pinocchio"))]
impl AccountInfoTrait for AccountInfo<'_> {
    fn key(&self) -> &Pubkey {
        self.key
    }
}

#[cfg(not(feature = "pinocchio"))]
impl AccountInfoTrait for &AccountInfo<'_> {
    fn key(&self) -> &Pubkey {
        self.key
    }
}
