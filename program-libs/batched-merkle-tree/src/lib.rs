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
pub(crate) use borsh::{BorshDeserialize, BorshSerialize};
// Pinocchio imports when pinocchio feature is enabled
#[cfg(feature = "pinocchio")]
pub(crate) use pinocchio::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvars::rent::Rent, sysvars::Sysvar,
};
// Solana program imports for non-pinocchio builds (default)
#[cfg(not(feature = "pinocchio"))]
pub(crate) use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::rent::Rent, sysvar::Sysvar,
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
