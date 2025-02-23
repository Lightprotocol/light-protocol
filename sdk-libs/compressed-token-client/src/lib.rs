//! Client library for interacting with the Compressed Token Program

pub mod instructions;

// We're also re-exporting helpers from the compressed-token program.
pub use instructions::{
    batch_compress, compress, create_compress_instruction, create_decompress_instruction,
    CompressParams, CompressedTokenError, DecompressParams,
};
pub use light_compressed_account::{
    compressed_account::{CompressedAccount, MerkleContext},
    instruction_data::compressed_proof::CompressedProof,
    TreeType,
};
pub use light_compressed_token::instruction;
pub use light_compressed_token::ErrorCode;
pub use light_compressed_token::{
    burn::sdk as burn_sdk, delegation::sdk as delegation_sdk, freeze::sdk as freeze_sdk,
    process_compress_spl_token_account::sdk as compress_spl_token_account_sdk,
    process_mint::mint_sdk, process_transfer::transfer_sdk,
};
pub use light_compressed_token::{get_token_pool_pda, ID as PROGRAM_ID};
pub use light_compressed_token::{
    process_transfer::{get_cpi_authority_pda, TokenTransferOutputData},
    token_data::{AccountState, TokenData},
};
pub use light_system_program::ID as LIGHT_SYSTEM_PROGRAM_ID;
