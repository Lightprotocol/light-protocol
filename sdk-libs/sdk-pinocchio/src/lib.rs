pub mod account;
pub mod address;
pub mod compressed_account;
pub mod cpi;
pub mod error;
pub mod hash_to_field_size;
pub mod instruction;

// Re-export commonly used items
// Re-export key types from modules
pub use account::LightAccount;
// Core types we'll need
pub use borsh::{BorshDeserialize, BorshSerialize};
pub use compressed_account::{
    CompressedAccountInfo, InAccountInfo, OutAccountInfo, PackedMerkleContext,
};
pub use cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs};
pub use instruction::{
    account_meta::CompressedAccountMeta,
    tree_info::{PackedAddressTreeInfo, PackedStateTreeInfo},
};
// Re-export discriminator functionality
pub use light_account_checks::discriminator::Discriminator as LightDiscriminator;
pub use light_hasher::DataHasher as LightHasher;
pub use light_hasher::{DataHasher, Poseidon}; // For backward compatibility
// Re-export derive macros
pub use light_sdk_macros::{LightDiscriminator, LightHasher};
// Re-export light-verifier for compatibility
pub use light_verifier;
use pinocchio::pubkey::Pubkey;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ValidityProof(pub Option<CompressedProof>);

impl ValidityProof {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}

impl From<CompressedProof> for ValidityProof {
    fn from(proof: CompressedProof) -> Self {
        Self(Some(proof))
    }
}

impl From<Option<CompressedProof>> for ValidityProof {
    fn from(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}
impl From<&CompressedProof> for ValidityProof {
    fn from(proof: &CompressedProof) -> Self {
        Self(Some(*proof))
    }
}

impl From<&Option<CompressedProof>> for ValidityProof {
    fn from(proof: &Option<CompressedProof>) -> Self {
        Self(*proof)
    }
}

#[allow(clippy::from_over_into)]
impl Into<Option<CompressedProof>> for ValidityProof {
    fn into(self) -> Option<CompressedProof> {
        self.0
    }
}

// Data structures copied from light-compressed-account to avoid dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

// impl From<ValidityProof> for Option<CompressedProof> {
//     fn from(proof: ValidityProof) -> Self {
//         Some(CompressedProof {
//             a: proof.0[0..32].try_into().unwrap(),
//             b: proof.0[32..96].try_into().unwrap(),
//             c: proof.0[96..128].try_into().unwrap(),
//         })
//     }
// }

// impl From<CompressedProof> for ValidityProof {
//     fn from(proof: CompressedProof) -> Self {
//         let mut bytes = [0u8; 128];
//         bytes[0..32].copy_from_slice(&proof.a);
//         bytes[32..96].copy_from_slice(&proof.b);
//         bytes[96..128].copy_from_slice(&proof.c);
//         Self(bytes)
//     }
// }

// // Conversion from light-verifier CompressedProof (used by light-sdk in tests)
// impl From<light_verifier::CompressedProof> for ValidityProof {
//     fn from(proof: light_verifier::CompressedProof) -> Self {
//         let mut bytes = [0u8; 128];
//         bytes[0..32].copy_from_slice(&proof.a);
//         bytes[32..96].copy_from_slice(&proof.b);
//         bytes[96..128].copy_from_slice(&proof.c);
//         Self(bytes)
//     }
// }

#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}
