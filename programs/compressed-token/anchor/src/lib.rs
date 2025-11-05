// Allow deprecated to suppress warnings from anchor_lang::AccountInfo::realloc
// which is used in the #[program] macro but we don't directly control
#![allow(deprecated)]

use anchor_lang::prelude::*;

pub mod constants;
pub mod process_compress_spl_token_account;
pub mod process_mint;
pub mod process_transfer;
use process_compress_spl_token_account::process_compress_spl_token_account;
pub mod spl_compression;
pub use light_ctoken_types::state::TokenData;
pub use process_mint::*;
pub mod delegation;
pub mod freeze;
pub mod instructions;
pub use instructions::*;
pub mod burn;
pub use burn::*;
pub mod batch_compress;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;

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
    use light_zero_copy::traits::ZeroCopyAt;
    use spl_compression::check_spl_token_pool_derivation_with_index;

    use super::*;

    /// This instruction creates a token pool for a given mint. Every spl mint
    /// can have one token pool. When a token is compressed the tokens are
    /// transferrred to the token pool, and their compressed equivalent is
    /// minted into a Merkle tree.
    pub fn create_token_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateTokenPoolInstruction<'info>>,
    ) -> Result<()> {
        instructions::create_token_pool::assert_mint_extensions(
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
        process_mint_to_or_compress::<MINT_TO>(
            ctx,
            public_keys.as_slice(),
            amounts.as_slice(),
            lamports,
            None,
            None,
        )
    }

    /// Batch compress tokens to an of recipients.
    pub fn batch_compress<'info>(
        ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let (inputs, _) = batch_compress::BatchCompressInstructionData::zero_copy_at(&inputs)
            .map_err(ProgramError::from)?;
        if inputs.amounts.is_some() && inputs.amount.is_some() {
            return Err(crate::ErrorCode::AmountsAndAmountProvided.into());
        }
        let amounts = if let Some(amount) = inputs.amount {
            vec![*amount; inputs.pubkeys.len()]
        } else if let Some(amounts) = inputs.amounts {
            amounts.to_vec()
        } else {
            return Err(crate::ErrorCode::NoAmount.into());
        };

        process_mint_to_or_compress::<COMPRESS>(
            ctx,
            inputs.pubkeys.as_slice(),
            amounts.as_slice(),
            inputs.lamports.map(|x| (*x).into()),
            Some(inputs.index),
            Some(inputs.bump),
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
        let mut inputs = inputs;
        // Borsh ignores excess bytes -> push bool false for with_transaction_hash field.
        inputs.extend_from_slice(&[0u8; 1]);
        let inputs: CompressedTokenInstructionDataTransfer =
            CompressedTokenInstructionDataTransfer::deserialize(&mut inputs.as_slice())?;
        // Only check CPI context if we're compressing or decompressing (modifying Solana account state)
        if inputs.compress_or_decompress_amount.is_some() {
            check_cpi_context(&inputs.cpi_context)?;
        }
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
    NoAmount,
    AmountsAndAmountProvided,
    #[msg("Cpi context set and set first is not usable with burn, compression(transfer ix) or decompress(transfer).")]
    CpiContextSetNotUsable,
    MintIsNone,
    InvalidMintPda,
    #[msg("Sum inputs mint indices not in ascending order.")]
    InputsOutOfOrder,
    #[msg("Sum check, too many mints (max 5).")]
    TooManyMints,
    InvalidExtensionType,
    InstructionDataExpectedDelegate,
    ZeroCopyExpectedDelegate,
    TokenDataTlvUnimplemented,
    // Mint Action specific errors
    #[msg("Mint action requires at least one action")]
    MintActionNoActionsProvided,
    #[msg("Missing mint signer account for SPL mint creation")]
    MintActionMissingSplMintSigner,
    #[msg("Missing system account configuration for mint action")]
    MintActionMissingSystemAccount,
    #[msg("Invalid mint bump seed provided")]
    MintActionInvalidMintBump,
    #[msg("Missing mint account for decompressed mint operations")]
    MintActionMissingMintAccount,
    #[msg("Missing token pool account for decompressed mint operations")]
    MintActionMissingTokenPoolAccount,
    #[msg("Missing token program for SPL operations")]
    MintActionMissingTokenProgram,
    #[msg("Mint account does not match expected mint")]
    MintAccountMismatch,
    #[msg("Invalid or missing authority for compression operation")]
    InvalidCompressAuthority,
    #[msg("Invalid queue index configuration")]
    MintActionInvalidQueueIndex,
    #[msg("Mint output serialization failed")]
    MintActionSerializationFailed,
    #[msg("Proof required for mint action but not provided")]
    MintActionProofMissing,
    #[msg("Unsupported mint action type")]
    MintActionUnsupportedActionType,
    #[msg("Metadata operations require decompressed mints")]
    MintActionMetadataNotDecompressed,
    #[msg("Missing metadata extension in mint")]
    MintActionMissingMetadataExtension,
    #[msg("Extension index out of bounds")]
    MintActionInvalidExtensionIndex,
    #[msg("Invalid metadata value encoding")]
    MintActionInvalidMetadataValue,
    #[msg("Invalid metadata key encoding")]
    MintActionInvalidMetadataKey,
    #[msg("Extension at index is not a TokenMetadata extension")]
    MintActionInvalidExtensionType,
    #[msg("Metadata key not found")]
    MintActionMetadataKeyNotFound,
    #[msg("Missing executing system accounts for mint action")]
    MintActionMissingExecutingAccounts,
    #[msg("Invalid mint authority for mint action")]
    MintActionInvalidMintAuthority,
    #[msg("Invalid mint PDA derivation in mint action")]
    MintActionInvalidMintPda,
    #[msg("Missing system accounts for queue index calculation")]
    MintActionMissingSystemAccountsForQueue,
    #[msg("Account data serialization failed in mint output")]
    MintActionOutputSerializationFailed,
    #[msg("Mint amount too large, would cause overflow")]
    MintActionAmountTooLarge,
    #[msg("Initial supply must be 0 for new mint creation")]
    MintActionInvalidInitialSupply,
    #[msg("Mint version not supported")]
    MintActionUnsupportedVersion,
    #[msg("New mint must start as compressed")]
    MintActionInvalidCompressionState,
    MintActionUnsupportedOperation,
    // Close account specific errors
    #[msg("Cannot close account with non-zero token balance")]
    NonNativeHasBalance,
    #[msg("Authority signature does not match expected owner")]
    OwnerMismatch,
    #[msg("Account is frozen and cannot perform this operation")]
    AccountFrozen,
    // Account creation specific errors
    #[msg("Account size insufficient for token account")]
    InsufficientAccountSize,
    #[msg("Account already initialized")]
    AlreadyInitialized,
    #[msg("Extension instruction data invalid")]
    InvalidExtensionInstructionData,
    #[msg("Lamports amount too large")]
    MintActionLamportsAmountTooLarge,
    #[msg("Invalid token program provided")]
    InvalidTokenProgram,
    // Transfer2 specific errors
    #[msg("Cannot access system accounts for CPI context write operations")]
    Transfer2CpiContextWriteInvalidAccess,
    #[msg("SOL pool operations not supported with CPI context write")]
    Transfer2CpiContextWriteWithSolPool,
    #[msg("Change account must not contain token data")]
    Transfer2InvalidChangeAccountData,
    #[msg("Cpi context expected but not provided.")]
    CpiContextExpected,
    #[msg("CPI accounts slice exceeds provided account infos")]
    CpiAccountsSliceOutOfBounds,
    // CompressAndClose specific errors
    #[msg("CompressAndClose requires a destination account for rent lamports")]
    CompressAndCloseDestinationMissing,
    #[msg("CompressAndClose requires an authority account")]
    CompressAndCloseAuthorityMissing,
    #[msg("CompressAndClose: Compressed token owner does not match expected owner")]
    CompressAndCloseInvalidOwner,
    #[msg("CompressAndClose: Compression amount must match the full token balance")]
    CompressAndCloseAmountMismatch,
    #[msg("CompressAndClose: Token account balance must match compressed output amount")]
    CompressAndCloseBalanceMismatch,
    #[msg("CompressAndClose: Compressed token must not have a delegate")]
    CompressAndCloseDelegateNotAllowed,
    #[msg("CompressAndClose: Invalid compressed token version")]
    CompressAndCloseInvalidVersion,
    #[msg("InvalidAddressTree")]
    InvalidAddressTree,
    #[msg("Too many compression transfers. Maximum 40 transfers allowed per instruction")]
    TooManyCompressionTransfers,
    #[msg("Missing fee payer for compressions-only operation")]
    CompressionsOnlyMissingFeePayer,
    #[msg("Missing CPI authority PDA for compressions-only operation")]
    CompressionsOnlyMissingCpiAuthority,
    #[msg("Cpi authority pda expected but not provided.")]
    ExpectedCpiAuthority,
    #[msg("InvalidRentSponsor")]
    InvalidRentSponsor,
    TooManyMintToRecipients,
    #[msg("Prefunding for exactly 1 epoch is not allowed due to epoch boundary timing risk. Use 0 or 2+ epochs.")]
    OneEpochPrefundingNotAllowed,
    #[msg("Duplicate mint index detected in inputs, outputs, or compressions")]
    DuplicateMint,
    #[msg("Invalid compressed mint address derivation")]
    MintActionInvalidCompressedMintAddress,
    #[msg("Invalid CPI context for create mint operation")]
    MintActionInvalidCpiContextForCreateMint,
    #[msg("Invalid address tree pubkey in CPI context")]
    MintActionInvalidCpiContextAddressTreePubkey,
    #[msg("CompressAndClose: Cannot use the same compressed output account for multiple closures")]
    CompressAndCloseDuplicateOutput,
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// Checks if CPI context usage is valid for the current instruction
/// Throws an error if cpi_context is Some and (set_context OR first_set_context is true)
pub fn check_cpi_context(cpi_context: &Option<CompressedCpiContext>) -> Result<()> {
    if let Some(ctx) = cpi_context {
        if ctx.set_context || ctx.first_set_context {
            return Err(ErrorCode::CpiContextSetNotUsable.into());
        }
    }
    Ok(())
}
