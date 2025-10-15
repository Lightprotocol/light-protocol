pub mod approve;
pub mod batch_compress;
pub mod claim;
pub mod close;
pub mod compress_and_close;
pub mod create_associated_token_account;
pub mod create_compressed_mint;
mod create_spl_mint;
pub mod create_token_account;
pub mod ctoken_accounts;
pub mod decompress_full;
pub mod mint_action;
pub mod mint_to_compressed;
pub mod transfer;
pub mod transfer2;
pub mod update_compressed_mint;
pub mod withdraw_funding_pool;

// Re-export all instruction utilities
pub use approve::{
    approve, create_approve_instruction, get_approve_instruction_account_metas, ApproveInputs,
    ApproveMetaConfig,
};
pub use batch_compress::{
    create_batch_compress_instruction, get_batch_compress_instruction_account_metas,
    BatchCompressInputs, BatchCompressMetaConfig, Recipient,
};
pub use claim::claim;
pub use compress_and_close::{
    compress_and_close_ctoken_accounts, compress_and_close_ctoken_accounts_with_indices,
    CompressAndCloseIndices,
};
pub use create_associated_token_account::*;
pub use create_compressed_mint::*;
pub use create_spl_mint::*;
pub use create_token_account::{
    create_compressible_token_account, create_token_account, CreateCompressibleTokenAccount,
};
pub use ctoken_accounts::*;
pub use decompress_full::{decompress_full_ctoken_accounts_with_indices, DecompressFullIndices};
pub use mint_action::{
    create_mint_action, create_mint_action_cpi, get_mint_action_instruction_account_metas,
    get_mint_action_instruction_account_metas_cpi_write, mint_action_cpi_write,
    CreateMintCpiWriteInputs, CreateMintInputs, MintActionInputs, MintActionInputsCpiWrite,
    MintActionMetaConfig, MintActionMetaConfigCpiWrite, MintActionType, MintToRecipient, TokenPool,
    WithMintCpiWriteInputs, WithMintInputs, MINT_ACTION_DISCRIMINATOR,
};
pub use mint_to_compressed::{
    create_mint_to_compressed_instruction, get_mint_to_compressed_instruction_account_metas,
    DecompressedMintConfig, MintToCompressedInputs, MintToCompressedMetaConfig,
};
pub use update_compressed_mint::{
    update_compressed_mint, update_compressed_mint_cpi, UpdateCompressedMintInputs,
    UPDATE_COMPRESSED_MINT_DISCRIMINATOR,
};
pub use withdraw_funding_pool::withdraw_funding_pool;

/// Derive token pool information for a given mint
pub fn derive_token_pool(mint: &solana_pubkey::Pubkey, index: u8) -> mint_action::TokenPool {
    let (pubkey, bump) = crate::token_pool::find_token_pool_pda_with_index(mint, index);
    mint_action::TokenPool {
        pubkey,
        bump,
        index,
    }
}
