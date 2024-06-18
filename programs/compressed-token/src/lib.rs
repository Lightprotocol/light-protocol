use anchor_lang::prelude::*;

pub mod constants;
pub mod process_mint;
pub mod process_transfer;
pub mod spl_compression;
pub use process_mint::*;
pub mod token_data;
pub use token_data::TokenData;
pub mod delegation;
pub mod freeze;
pub mod instructions;
pub use instructions::*;
pub mod burn;

use crate::process_transfer::CompressedTokenInstructionDataTransfer;
declare_id!("HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light-compressed-token",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

#[program]
pub mod light_compressed_token {

    use super::*;

    /// This instruction expects a mint account to be created in a separate
    /// token program instruction with token authority as mint authority. This
    /// instruction creates a token pool account for that mint owned by token
    /// authority.
    pub fn create_mint<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMintInstruction<'info>>,
    ) -> Result<()> {
        if ctx.accounts.token_pool_pda.is_native() {
            unimplemented!("Compressed wrapped SOL is not supported.");
        }
        Ok(())
    }

    /// Mints tokens from an spl token mint to a list of compressed accounts.
    /// Minted tokens are transferred to a pool account owned by the compressed
    /// token program. The instruction creates one compressed output account for
    /// every amount and pubkey input pair one output compressed account.
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

    pub fn approve<'info>(
        ctx: Context<'_, '_, '_, 'info, ApproveOrRevokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        delegation::process_approve(ctx, inputs)
    }

    pub fn revoke<'info>(
        ctx: Context<'_, '_, '_, 'info, ApproveOrRevokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        delegation::process_revoke(ctx, inputs)
    }

    pub fn freeze<'info>(
        ctx: Context<'_, '_, '_, 'info, FreezeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        freeze::process_freeze_or_thaw::<false, true>(ctx, inputs)
    }

    pub fn thaw<'info>(
        ctx: Context<'_, '_, '_, 'info, FreezeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        freeze::process_freeze_or_thaw::<true, false>(ctx, inputs)
    }

    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, ApproveOrRevokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        burn::process_burn(ctx, inputs)
    }

    /// This function is a stub to allow Anchor to include the input types in
    /// the IDL. It should not be included in production builds nor be called in
    /// practice.
    #[cfg(feature = "idl-build")]
    pub fn stub_idl_build<'info>(
        _ctx: Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
        _inputs1: CompressedTokenInstructionDataTransfer,
        _inputs2: TokenData,
    ) -> Result<()> {
        Err(ErrorCode::InstructionNotCallable.into())
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
    #[msg("HeapMemoryCheckFailed")]
    HeapMemoryCheckFailed,
    #[msg("The instruction is not callable")]
    InstructionNotCallable,
    #[msg("ArithmeticUnderflow")]
    ArithmeticUnderflow,
    #[msg("InvalidDelegate")]
    InvalidDelegate,
}
