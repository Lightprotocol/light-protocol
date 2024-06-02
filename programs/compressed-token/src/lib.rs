use anchor_lang::prelude::*;

pub mod constants;
pub mod process_mint;
pub mod process_transfer;
pub mod spl_compression;
pub use process_mint::*;
pub use process_transfer::*;
pub mod token_data;

declare_id!("HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light-compressed-token",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

#[constant]
pub const PROGRAM_ID: &str = "HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN";

#[program]
pub mod light_compressed_token {

    use super::*;

    /// This instruction expects a mint account to be created in a separate
    /// token program instruction with token authority as mint authority. This
    /// instruction creates a token pool account for that mint owned by token
    /// authority.
    pub fn create_mint<'info>(
        _ctx: Context<'_, '_, '_, 'info, CreateMintInstruction<'info>>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn mint_to<'info>(
        ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
        public_keys: Vec<Pubkey>,
        amounts: Vec<u64>,
    ) -> Result<()> {
        process_mint_to(ctx, public_keys, amounts)
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        process_transfer::process_transfer(ctx, inputs)
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("public keys and amounts must be of same length")]
    PublicKeyAmountMissmatch,
    #[msg("SignerCheckFailed")]
    SignerCheckFailed,
    #[msg("ComputeInputSumFailed")]
    ComputeInputSumFailed,
    #[msg("ComputeOutputSumFailed")]
    ComputeOutputSumFailed,
    #[msg("ComputeCompressSumFailed")]
    ComputeCompressSumFailed,
    #[msg("ComputeDecompressSumFailed")]
    ComputeDecompressSumFailed,
    #[msg("SumCheckFailed")]
    SumCheckFailed,
    #[msg("DecompressRecipientUndefinedForDecompress")]
    DecompressRecipientUndefinedForDecompress,
    #[msg("CompressedPdaUndefinedForDecompress")]
    CompressedPdaUndefinedForDecompress,
    #[msg("DeCompressAmountUndefinedForDecompress")]
    DeCompressAmountUndefinedForDecompress,
    #[msg("CompressedPdaUndefinedForCompress")]
    CompressedPdaUndefinedForCompress,
    #[msg("DeCompressAmountUndefinedForCompress")]
    DeCompressAmountUndefinedForCompress,
    #[msg("DelegateUndefined while delegated amount is defined")]
    DelegateUndefined,
    #[msg("DelegateSignerCheckFailed")]
    DelegateSignerCheckFailed,
    #[msg("SplTokenSupplyMismatch")]
    SplTokenSupplyMismatch,
}
