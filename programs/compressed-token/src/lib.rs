use anchor_lang::prelude::*;

pub mod constants;
pub mod process_compress_spl_token_account;
pub mod process_mint;
pub mod process_transfer;
use process_compress_spl_token_account::process_compress_spl_token_account;
pub mod spl_compression;
pub use process_mint::*;
pub mod token_data;
pub use token_data::TokenData;
pub mod delegation;
pub mod freeze;
pub mod instructions;
pub use instructions::*;
pub mod burn;
pub use burn::*;
pub mod batch_compress;
use light_utils::instruction::cpi_context::CompressedCpiContext;

use crate::process_transfer::CompressedTokenInstructionDataTransfer;
declare_id!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

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

    use constants::{NOT_FROZEN, NUM_MAX_POOL_ACCOUNTS};
    use light_zero_copy::borsh::Deserialize;
    use spl_compression::check_spl_token_pool_derivation_with_index;

    use super::*;

    /// This instruction creates a token pool for a given mint. Every spl mint
    /// can have one token pool. When a token is compressed the tokens are
    /// transferrred to the token pool, and their compressed equivalent is
    /// minted into a Merkle tree.
    pub fn create_token_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateTokenPoolInstruction<'info>>,
    ) -> Result<()> {
        create_token_pool::assert_mint_extensions(
            &ctx.accounts.mint.to_account_info().try_borrow_data()?,
        )
    }

    /// This instruction creates an additional token pool for a given mint.
    /// The maximum number of token pools per mint is 5.
    pub fn add_token_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, AddTokenPoolInstruction<'info>>,
        token_pool_index: u8,
    ) -> Result<()> {
        if token_pool_index >= NUM_MAX_POOL_ACCOUNTS {
            return err!(ErrorCode::InvalidTokenPoolBump);
        }
        // Check that token pool account with previous bump already exists.
        check_spl_token_pool_derivation_with_index(
            &ctx.accounts.mint.key().to_bytes(),
            &ctx.accounts.existing_token_pool_pda.key(),
            &[token_pool_index.saturating_sub(1)],
        )
    }

    /// Mints tokens from an spl token mint to a list of compressed accounts.
    /// Minted tokens are transferred to a pool account owned by the compressed
    /// token program. The instruction creates one compressed output account for
    /// every amount and pubkey input pair. A constant amount of lamports can be
    /// transferred to each output account to enable. A use case to add lamports
    /// to a compressed token account is to prevent spam. This is the only way
    /// to add lamports to a compressed token account.
    pub fn mint_to<'info>(
        ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
        public_keys: Vec<Pubkey>,
        amounts: Vec<u64>,
        lamports: Option<u64>,
    ) -> Result<()> {
        process_mint_to::<MINT_TO>(ctx, public_keys.as_slice(), amounts.as_slice(), lamports)
    }

    /// Batch compress tokens to a list of compressed accounts.
    pub fn batch_compress<'info>(
        ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let (inputs, _) = batch_compress::BatchCompressInstructionData::zero_copy_at(&inputs)
            .map_err(ProgramError::from)?;

        // TODO: make types cleaner for example change types in the remaining code to match these.
        process_mint_to::<COMPRESS>(
            ctx,
            inputs
                .pubkeys
                .iter()
                .map(|x| (*x).into())
                .collect::<Vec<Pubkey>>()
                .as_slice(),
            inputs
                .amounts
                .iter()
                .map(|x| (*x).into())
                .collect::<Vec<u64>>()
                .as_slice(),
            inputs.lamports.map(|x| u64::from(*x)),
        )
    }

    /// Compresses the balance of an spl token account sub an optional remaining
    /// amount. This instruction does not close the spl token account. To close
    /// the account bundle a close spl account instruction in your transaction.
    pub fn compress_spl_token_account<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
        owner: Pubkey,
        remaining_amount: Option<u64>,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        process_compress_spl_token_account(ctx, owner, remaining_amount, cpi_context)
    }

    /// Transfers compressed tokens from one account to another. All accounts
    /// must be of the same mint. Additional spl tokens can be compressed or
    /// decompressed. In one transaction only compression or decompression is
    /// possible. Lamports can be transferred alongside tokens. If output token
    /// accounts specify less lamports than inputs the remaining lamports are
    /// transferred to an output compressed account. Signer must be owner or
    /// delegate. If a delegated token account is transferred the delegate is
    /// not preserved.
    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: CompressedTokenInstructionDataTransfer =
            CompressedTokenInstructionDataTransfer::deserialize(&mut inputs.as_slice())?;
        process_transfer::process_transfer(ctx, inputs)
    }

    /// Delegates an amount to a delegate. A compressed token account is either
    /// completely delegated or not. Prior delegates are not preserved. Cannot
    /// be called by a delegate.
    /// The instruction creates two output accounts:
    /// 1. one account with delegated amount
    /// 2. one account with remaining(change) amount
    pub fn approve<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        delegation::process_approve(ctx, inputs)
    }

    /// Revokes a delegation. The instruction merges all inputs into one output
    /// account. Cannot be called by a delegate. Delegates are not preserved.
    pub fn revoke<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        delegation::process_revoke(ctx, inputs)
    }

    /// Freezes compressed token accounts. Inputs must not be frozen. Creates as
    /// many outputs as inputs. Balances and delegates are preserved.
    pub fn freeze<'info>(
        ctx: Context<'_, '_, '_, 'info, FreezeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        // Inputs are not frozen, outputs are frozen.
        freeze::process_freeze_or_thaw::<NOT_FROZEN, true>(ctx, inputs)
    }

    /// Thaws frozen compressed token accounts. Inputs must be frozen. Creates
    /// as many outputs as inputs. Balances and delegates are preserved.
    pub fn thaw<'info>(
        ctx: Context<'_, '_, '_, 'info, FreezeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        // Inputs are frozen, outputs are not frozen.
        freeze::process_freeze_or_thaw::<true, NOT_FROZEN>(ctx, inputs)
    }

    /// Burns compressed tokens and spl tokens from the pool account. Delegates
    /// can burn tokens. The output compressed token account remains delegated.
    /// Creates one output compressed token account.
    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, BurnInstruction<'info>>,
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
    #[msg("DelegateSignerCheckFailed")]
    DelegateSignerCheckFailed,
    #[msg("Minted amount greater than u64::MAX")]
    MintTooLarge,
    #[msg("SplTokenSupplyMismatch")]
    SplTokenSupplyMismatch,
    #[msg("HeapMemoryCheckFailed")]
    HeapMemoryCheckFailed,
    #[msg("The instruction is not callable")]
    InstructionNotCallable,
    #[msg("ArithmeticUnderflow")]
    ArithmeticUnderflow,
    #[msg("HashToFieldError")]
    HashToFieldError,
    #[msg("Expected the authority to be also a mint authority")]
    InvalidAuthorityMint,
    #[msg("Provided authority is not the freeze authority")]
    InvalidFreezeAuthority,
    InvalidDelegateIndex,
    TokenPoolPdaUndefined,
    #[msg("Compress or decompress recipient is the same account as the token pool pda.")]
    IsTokenPoolPda,
    InvalidTokenPoolPda,
    NoInputTokenAccountsProvided,
    NoInputsProvided,
    MintHasNoFreezeAuthority,
    MintWithInvalidExtension,
    #[msg("The token account balance is less than the remaining amount.")]
    InsufficientTokenAccountBalance,
    #[msg("Max number of token pools reached.")]
    InvalidTokenPoolBump,
    FailedToDecompress,
    FailedToBurnSplTokensFromTokenPool,
    NoMatchingBumpFound,
}
