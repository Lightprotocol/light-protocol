pub mod account;
pub mod address;
pub mod cpi;
pub mod error;
pub mod instruction;

pub use account::LightAccount;
pub use borsh::{BorshDeserialize, BorshSerialize};
pub use cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs};
pub use instruction::{
    account_meta::CompressedAccountMeta,
    tree_info::{PackedAddressTreeInfo, PackedStateTreeInfo},
};
// Re-export discriminator functionality
pub use light_account_checks::discriminator::Discriminator as LightDiscriminator;
// Re-export derive macros
pub use light_compressed_account::{
    self, instruction_data::compressed_proof::ValidityProof, instruction_data::data::*,
};
pub use light_hasher::{DataHasher as LightHasher, DataHasher, Poseidon};
pub use light_sdk_macros::{LightDiscriminator, LightHasher};
use pinocchio::pubkey::Pubkey;

pub mod hash_to_field_size {
    pub use light_hasher::hash_to_field_size::{
        hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be,
        hashv_to_bn254_field_size_be_const_array, HashToFieldSize,
    };
}

// Constants
/// Seed of the CPI authority.
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

/// ID of the account-compression program.
pub const PROGRAM_ID_ACCOUNT_COMPRESSION: Pubkey = [
    55, 8, 217, 140, 65, 94, 42, 215, 32, 189, 184, 135, 142, 143, 219, 27, 224, 96, 152, 85, 129,
    220, 130, 145, 39, 245, 180, 186, 206, 148, 10, 237,
];
pub const PROGRAM_ID_NOOP: Pubkey = [
    132, 155, 207, 4, 208, 227, 48, 117, 105, 194, 163, 167, 98, 204, 61, 138, 137, 185, 222, 182,
    70, 182, 113, 154, 85, 91, 240, 94, 151, 221, 190, 139,
];
/// ID of the light-system program.
pub const PROGRAM_ID_LIGHT_SYSTEM: Pubkey = [
    6, 167, 85, 248, 33, 57, 5, 77, 68, 36, 177, 90, 240, 196, 48, 207, 47, 75, 127, 152, 121, 58,
    218, 18, 82, 212, 143, 54, 102, 198, 203, 206,
];

// Macro for finding CPI signer
#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        pinocchio::pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id)
    };
}
