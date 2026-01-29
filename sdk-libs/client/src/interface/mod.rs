//! Client utilities for hot/cold account handling.

pub mod account_interface;
pub mod create_accounts_proof;
pub mod decompress_mint;
pub mod initialize_config;
pub mod instructions;
pub mod light_program_interface;
pub mod load_accounts;
pub mod pack;
pub mod tx_size;

pub use account_interface::{AccountInterface, AccountInterfaceError, TokenAccountInterface};
pub use create_accounts_proof::{
    get_create_accounts_proof, CreateAccountsProofError, CreateAccountsProofInput,
    CreateAccountsProofResult,
};
pub use decompress_mint::{
    DecompressMintError, MintInterface, MintState, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
pub use initialize_config::InitializeRentFreeConfig;
pub use light_account::LightConfig;
pub use light_program_interface::{
    all_hot, any_cold, discriminator, matches_discriminator, AccountSpec, AccountToFetch,
    ColdContext, LightProgramInterface, PdaSpec,
};
pub use light_sdk_types::interface::CreateAccountsProof;
pub use light_token::compat::TokenData;
pub use load_accounts::{create_load_instructions, LoadAccountsError};
pub use pack::{pack_proof, PackError, PackedProofResult};
pub use solana_account::Account;
pub use tx_size::{split_by_tx_size, InstructionTooLargeError, PACKET_DATA_SIZE};
