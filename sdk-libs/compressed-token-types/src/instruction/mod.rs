pub mod batch_compress;
pub mod burn;
pub mod delegation;
pub mod freeze;
pub mod generic;
pub mod mint_to;
pub mod transfer;
pub mod update_compressed_mint;

// Re-export ValidityProof same as in light-sdk
pub use batch_compress::*;
pub use burn::*;
pub use delegation::*;
pub use freeze::*;
// Export the generic instruction with an alias as the main type
pub use generic::CompressedTokenInstructionData;
pub use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use mint_to::*;
// Re-export all instruction data types
pub use transfer::*;
pub use update_compressed_mint::*;
