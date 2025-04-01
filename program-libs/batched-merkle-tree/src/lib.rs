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
#[cfg(feature = "anchor")]
pub(crate) use anchor_lang::{
    AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize,
};
#[cfg(not(feature = "anchor"))]
pub(crate) use borsh::{BorshDeserialize, BorshSerialize};

// Solana program imports
#[cfg(feature = "solana")]
pub(crate) use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::rent::Rent, sysvar::Sysvar,
};

// Anchor imports when anchor feature is enabled but solana is not
#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
pub(crate) use anchor_lang::{
    self,
    prelude::msg,
    prelude::AccountInfo,
    prelude::ProgramError,
    prelude::Pubkey,
    solana_program::sysvar::{rent::Rent, Sysvar},
};

// Pinocchio imports when pinocchio feature is enabled but others are not
#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "anchor")
))]
pub(crate) use pinocchio::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvars::rent::Rent, sysvars::Sysvar,
};

#[allow(unused)]
trait AccountInfoTrait {
    fn key(&self) -> &Pubkey;
}

#[cfg(any(feature = "solana", feature = "anchor"))]
impl AccountInfoTrait for AccountInfo<'_> {
    fn key(&self) -> &Pubkey {
        self.key
    }
}
#[cfg(any(feature = "solana", feature = "anchor"))]
impl AccountInfoTrait for &AccountInfo<'_> {
    fn key(&self) -> &Pubkey {
        self.key
    }
}
