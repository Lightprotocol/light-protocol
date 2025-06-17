pub mod transfer;
pub mod burn;
pub mod freeze;
pub mod delegation;
pub mod batch_compress;
pub mod mint_to;
pub mod generic;

// Re-export ValidityProof same as in light-sdk
pub use light_compressed_account::instruction_data::compressed_proof::ValidityProof;

// Re-export all instruction data types
pub use transfer::*;
pub use burn::*;
pub use freeze::*;
pub use delegation::*;
pub use batch_compress::*;
pub use mint_to::*;

// Export the generic instruction with an alias as the main type
pub use generic::CompressedTokenInstructionData;