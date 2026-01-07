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
pub use light_ctoken_interface::state::TokenData;
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
    use instructions::create_token_pool::restricted_seed;
    use light_ctoken_interface::is_valid_spl_interface_pda;
    use light_zero_copy::traits::ZeroCopyAt;

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
        )?;
        // Initialize the token account via CPI (Anchor's init constraint only allocated space)
        instructions::create_token_pool::initialize_token_account(
            &ctx.accounts.token_pool_pda,
            &ctx.accounts.mint,
            &ctx.accounts.cpi_authority_pda,
            &ctx.accounts.token_program.to_account_info(),
        )
    }

    /// This instruction creates an additional token pool for a given mint.
    /// The maximum number of token pools per mint is 5.
    /// For mints with restricted extensions, uses restricted PDA derivation.
    pub fn add_token_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, AddTokenPoolInstruction<'info>>,
        token_pool_index: u8,
    ) -> Result<()> {
        if token_pool_index >= NUM_MAX_POOL_ACCOUNTS {
            return err!(ErrorCode::InvalidTokenPoolBump);
        }
        // Check that token pool account with previous index already exists.
        // Use the same restricted derivation as the new pool.
        let is_restricted = !restricted_seed(&ctx.accounts.mint).is_empty();
        let prev_index = token_pool_index.saturating_sub(1);
        if !is_valid_spl_interface_pda(
            &ctx.accounts.mint.key().to_bytes(),
            &ctx.accounts.existing_token_pool_pda.key(),
            prev_index,
            None,
            is_restricted,
        ) {
            return err!(ErrorCode::InvalidTokenPoolPda);
        }
        // Initialize the token account via CPI (Anchor's init constraint only allocated space)
        instructions::create_token_pool::initialize_token_account(
            &ctx.accounts.token_pool_pda,
            &ctx.accounts.mint,
            &ctx.accounts.cpi_authority_pda,
            &ctx.accounts.token_program.to_account_info(),
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
    PublicKeyAmountMissmatch, // 6000
    #[msg("ComputeInputSumFailed")]
    ComputeInputSumFailed, // 6001
    #[msg("ComputeOutputSumFailed")]
    ComputeOutputSumFailed, // 6002
    #[msg("ComputeCompressSumFailed")]
    ComputeCompressSumFailed, // 6003
    #[msg("ComputeDecompressSumFailed")]
    ComputeDecompressSumFailed, // 6004
    #[msg("SumCheckFailed")]
    SumCheckFailed, // 6005
    #[msg("DecompressRecipientUndefinedForDecompress")]
    DecompressRecipientUndefinedForDecompress, // 6006
    #[msg("CompressedPdaUndefinedForDecompress")]
    CompressedPdaUndefinedForDecompress, // 6007
    #[msg("DeCompressAmountUndefinedForDecompress")]
    DeCompressAmountUndefinedForDecompress, // 6008
    #[msg("CompressedPdaUndefinedForCompress")]
    CompressedPdaUndefinedForCompress, // 6009
    #[msg("DeCompressAmountUndefinedForCompress")]
    DeCompressAmountUndefinedForCompress, // 6010
    #[msg("DelegateSignerCheckFailed")]
    DelegateSignerCheckFailed, // 6011
    #[msg("Minted amount greater than u64::MAX")]
    MintTooLarge, // 6012
    #[msg("SplTokenSupplyMismatch")]
    SplTokenSupplyMismatch, // 6013
    #[msg("HeapMemoryCheckFailed")]
    HeapMemoryCheckFailed, // 6014
    #[msg("The instruction is not callable")]
    InstructionNotCallable, // 6015
    #[msg("ArithmeticUnderflow")]
    ArithmeticUnderflow, // 6016
    #[msg("HashToFieldError")]
    HashToFieldError, // 6017
    #[msg("Expected the authority to be also a mint authority")]
    InvalidAuthorityMint, // 6018
    #[msg("Provided authority is not the freeze authority")]
    InvalidFreezeAuthority, // 6019
    InvalidDelegateIndex,  // 6020
    TokenPoolPdaUndefined, // 6021
    #[msg("Compress or decompress recipient is the same account as the token pool pda.")]
    IsTokenPoolPda, // 6022
    InvalidTokenPoolPda,   // 6023
    NoInputTokenAccountsProvided, // 6024
    NoInputsProvided,      // 6025
    MintHasNoFreezeAuthority, // 6026
    MintWithInvalidExtension, // 6027
    #[msg("The token account balance is less than the remaining amount.")]
    InsufficientTokenAccountBalance, // 6028
    #[msg("Max number of token pools reached.")]
    InvalidTokenPoolBump, // 6029
    FailedToDecompress,    // 6030
    FailedToBurnSplTokensFromTokenPool, // 6031
    NoMatchingBumpFound,   // 6032
    NoAmount,              // 6033
    AmountsAndAmountProvided, // 6034
    #[msg("Cpi context set and set first is not usable with burn, compression(transfer ix) or decompress(transfer).")]
    CpiContextSetNotUsable, // 6035
    MintIsNone,            // 6036
    InvalidMintPda,        // 6037
    #[msg("Sum inputs mint indices not in ascending order.")]
    InputsOutOfOrder, // 6038
    #[msg("Sum check, too many mints (max 5).")]
    TooManyMints, // 6039
    InvalidExtensionType,  // 6040
    InstructionDataExpectedDelegate, // 6041
    ZeroCopyExpectedDelegate, // 6042
    #[msg("Unsupported TLV extension type - only CompressedOnly is currently implemented")]
    UnsupportedTlvExtensionType, // 6043
    // Mint Action specific errors
    #[msg("Mint action requires at least one action")]
    MintActionNoActionsProvided, // 6044
    #[msg("Missing mint signer account for SPL mint creation")]
    MintActionMissingSplMintSigner, // 6045
    #[msg("Missing system account configuration for mint action")]
    MintActionMissingSystemAccount, // 6046
    #[msg("Invalid mint bump seed provided")]
    MintActionInvalidMintBump, // 6047
    #[msg("Missing mint account for decompressed mint operations")]
    MintActionMissingMintAccount, // 6048
    #[msg("Missing token pool account for decompressed mint operations")]
    MintActionMissingTokenPoolAccount, // 6049
    #[msg("Missing token program for SPL operations")]
    MintActionMissingTokenProgram, // 6050
    #[msg("Mint account does not match expected mint")]
    MintAccountMismatch, // 6051
    #[msg("Invalid or missing authority for compression operation")]
    InvalidCompressAuthority, // 6052
    #[msg("Invalid queue index configuration")]
    MintActionInvalidQueueIndex, // 6053
    #[msg("Mint output serialization failed")]
    MintActionSerializationFailed, // 6054
    #[msg("Proof required for mint action but not provided")]
    MintActionProofMissing, // 6055
    #[msg("Unsupported mint action type")]
    MintActionUnsupportedActionType, // 6056
    #[msg("Metadata operations require decompressed mints")]
    MintActionMetadataNotDecompressed, // 6057
    #[msg("Missing metadata extension in mint")]
    MintActionMissingMetadataExtension, // 6058
    #[msg("Extension index out of bounds")]
    MintActionInvalidExtensionIndex, // 6059
    #[msg("Invalid metadata value encoding")]
    MintActionInvalidMetadataValue, // 6060
    #[msg("Invalid metadata key encoding")]
    MintActionInvalidMetadataKey, // 6061
    #[msg("Extension at index is not a TokenMetadata extension")]
    MintActionInvalidExtensionType, // 6062
    #[msg("Metadata key not found")]
    MintActionMetadataKeyNotFound, // 6063
    #[msg("Missing executing system accounts for mint action")]
    MintActionMissingExecutingAccounts, // 6064
    #[msg("Invalid mint authority for mint action")]
    MintActionInvalidMintAuthority, // 6065
    #[msg("Invalid mint PDA derivation in mint action")]
    MintActionInvalidMintPda, // 6066
    #[msg("Missing system accounts for queue index calculation")]
    MintActionMissingSystemAccountsForQueue, // 6067
    #[msg("Account data serialization failed in mint output")]
    MintActionOutputSerializationFailed, // 6068
    #[msg("Mint amount too large, would cause overflow")]
    MintActionAmountTooLarge, // 6069
    #[msg("Initial supply must be 0 for new mint creation")]
    MintActionInvalidInitialSupply, // 6070
    #[msg("Mint version not supported")]
    MintActionUnsupportedVersion, // 6071
    #[msg("New mint must start as compressed")]
    MintActionInvalidCompressionState, // 6072
    MintActionUnsupportedOperation, // 6073
    // Close account specific errors
    #[msg("Cannot close account with non-zero token balance")]
    NonNativeHasBalance, // 6074
    #[msg("Authority signature does not match expected owner")]
    OwnerMismatch, // 6075
    #[msg("Account is frozen and cannot perform this operation")]
    AccountFrozen, // 6076
    // Account creation specific errors
    #[msg("Account size insufficient for token account")]
    InsufficientAccountSize, // 6077
    #[msg("Account already initialized")]
    AlreadyInitialized, // 6078
    #[msg("Extension instruction data invalid")]
    InvalidExtensionInstructionData, // 6079
    #[msg("Lamports amount too large")]
    MintActionLamportsAmountTooLarge, // 6080
    #[msg("Invalid token program provided")]
    InvalidTokenProgram, // 6081
    // Transfer2 specific errors
    #[msg("Cannot access system accounts for CPI context write operations")]
    Transfer2CpiContextWriteInvalidAccess, // 6082
    #[msg("SOL pool operations not supported with CPI context write")]
    Transfer2CpiContextWriteWithSolPool, // 6083
    #[msg("Change account must not contain token data")]
    Transfer2InvalidChangeAccountData, // 6084
    #[msg("Cpi context expected but not provided.")]
    CpiContextExpected, // 6085
    #[msg("CPI accounts slice exceeds provided account infos")]
    CpiAccountsSliceOutOfBounds, // 6086
    // CompressAndClose specific errors
    #[msg("CompressAndClose requires a destination account for rent lamports")]
    CompressAndCloseDestinationMissing, // 6087
    #[msg("CompressAndClose requires an authority account")]
    CompressAndCloseAuthorityMissing, // 6088
    #[msg("CompressAndClose: Compressed token owner does not match expected owner")]
    CompressAndCloseInvalidOwner, // 6089
    #[msg("CompressAndClose: Compression amount must match the full token balance")]
    CompressAndCloseAmountMismatch, // 6090
    #[msg("CompressAndClose: Token account balance must match compressed output amount")]
    CompressAndCloseBalanceMismatch, // 6091
    #[msg("CompressAndClose: Compressed token must not have a delegate")]
    CompressAndCloseDelegateNotAllowed, // 6092
    #[msg("CompressAndClose: Invalid compressed token version")]
    CompressAndCloseInvalidVersion, // 6093
    #[msg("InvalidAddressTree")]
    InvalidAddressTree, // 6094
    #[msg("Too many compression transfers. Maximum 32 transfers allowed per instruction")]
    TooManyCompressionTransfers, // 6095
    #[msg("Missing fee payer for compressions-only operation")]
    CompressionsOnlyMissingFeePayer, // 6096
    #[msg("Missing CPI authority PDA for compressions-only operation")]
    CompressionsOnlyMissingCpiAuthority, // 6097
    #[msg("Cpi authority pda expected but not provided.")]
    ExpectedCpiAuthority, // 6098
    #[msg("InvalidRentSponsor")]
    InvalidRentSponsor, // 6099
    TooManyMintToRecipients, // 6100
    #[msg("Prefunding for exactly 1 epoch is not allowed due to epoch boundary timing risk. Use 0 or 2+ epochs.")]
    OneEpochPrefundingNotAllowed, // 6101
    #[msg("Duplicate mint index detected in inputs, outputs, or compressions")]
    DuplicateMint, // 6102
    #[msg("Invalid compressed mint address derivation")]
    MintActionInvalidCompressedMintAddress, // 6103
    #[msg("Invalid CPI context for create mint operation")]
    MintActionInvalidCpiContextForCreateMint, // 6104
    #[msg("Invalid address tree pubkey in CPI context")]
    MintActionInvalidCpiContextAddressTreePubkey, // 6105
    #[msg("CompressAndClose: Cannot use the same compressed output account for multiple closures")]
    CompressAndCloseDuplicateOutput, // 6106
    #[msg(
        "CompressAndClose by compression authority requires compressed token account in outputs"
    )]
    CompressAndCloseOutputMissing, // 6107
    // CMint (decompressed compressed mint) specific errors
    #[msg("Missing mint signer account for mint action")]
    MintActionMissingMintSigner, // 6108
    #[msg("Missing CMint account for decompress mint action")]
    MintActionMissingCMintAccount, // 6109
    #[msg("CMint account already exists")]
    CMintAlreadyExists, // 6110
    #[msg("Invalid CMint account owner")]
    InvalidCMintOwner, // 6111
    #[msg("Failed to deserialize CMint account data")]
    CMintDeserializationFailed, // 6112
    #[msg("Failed to resize CMint account")]
    CMintResizeFailed, // 6113
    // CMint Compressibility errors
    #[msg("Invalid rent payment - must be >= 2 (CMint is always compressible)")]
    InvalidRentPayment, // 6114
    #[msg("Missing compressible config account for CMint")]
    MissingCompressibleConfig, // 6115
    #[msg("Missing rent sponsor account for CMint")]
    MissingRentSponsor, // 6116
    #[msg("Rent payment exceeds max funded epochs")]
    RentPaymentExceedsMax, // 6117
    #[msg("Write top-up exceeds maximum allowed")]
    WriteTopUpExceedsMaximum, // 6118
    #[msg("Failed to calculate CMint top-up amount")]
    CMintTopUpCalculationFailed, // 6119
    // CompressAndCloseCMint specific errors
    #[msg("CMint is not decompressed")]
    CMintNotDecompressed, // 6120
    #[msg("CMint is missing Compressible extension")]
    CMintMissingCompressibleExtension, // 6121
    #[msg("CMint is not compressible (rent not expired)")]
    CMintNotCompressible, // 6122
    #[msg("Cannot combine DecompressMint and CompressAndCloseCMint in same instruction")]
    CannotDecompressAndCloseInSameInstruction, // 6123
    #[msg("CMint account does not match compressed_mint.metadata.mint")]
    InvalidCMintAccount, // 6124
    #[msg("Mint data required in instruction when not decompressed")]
    MintDataRequired, // 6125
    // Extension validation errors
    #[msg("Invalid mint account data")]
    InvalidMint, // 6126
    #[msg("Token operations blocked - mint is paused")]
    MintPaused, // 6127
    #[msg("Mint account required for transfer when account has PausableAccount extension")]
    MintRequiredForTransfer, // 6128
    #[msg("Non-zero transfer fees are not supported")]
    NonZeroTransferFeeNotSupported, // 6129
    #[msg("Transfer hooks with non-nil program_id are not supported")]
    TransferHookNotSupported, // 6130
    #[msg("Mint has extensions that require compression_only mode")]
    CompressionOnlyRequired, // 6131
    #[msg("CompressAndClose: Compressed token mint does not match source token account mint")]
    CompressAndCloseInvalidMint, // 6132
    #[msg("CompressAndClose: Missing required CompressedOnly extension in output TLV")]
    CompressAndCloseMissingCompressedOnlyExtension, // 6133
    #[msg("CompressAndClose: CompressedOnly mint_account_index must be 0")]
    CompressAndCloseInvalidMintAccountIndex, // 6134
    #[msg(
        "CompressAndClose: Delegated amount mismatch between ctoken and CompressedOnly extension"
    )]
    CompressAndCloseDelegatedAmountMismatch, // 6135
    #[msg("CompressAndClose: Delegate mismatch between ctoken and compressed token output")]
    CompressAndCloseInvalidDelegate, // 6136
    #[msg("CompressAndClose: Withheld transfer fee mismatch")]
    CompressAndCloseWithheldFeeMismatch, // 6137
    #[msg("CompressAndClose: Frozen state mismatch")]
    CompressAndCloseFrozenMismatch, // 6138
    #[msg("TLV extensions require version 3 (ShaFlat)")]
    TlvRequiresVersion3, // 6139
    #[msg("CToken account has extensions that cannot be compressed. Only Compressible extension or no extensions allowed.")]
    CTokenHasDisallowedExtensions, // 6140
    #[msg("CompressAndClose: rent_sponsor_is_signer flag does not match actual signer")]
    RentSponsorIsSignerMismatch, // 6141
    #[msg("Mint has restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook, DefaultAccountState) must not create compressed token accounts.")]
    MintHasRestrictedExtensions, // 6142
    #[msg("Decompress: CToken delegate does not match input compressed account delegate")]
    DecompressDelegateMismatch, // 6143
    #[msg("Mint cache capacity exceeded (max 5 unique mints)")]
    MintCacheCapacityExceeded, // 6144
    #[msg("in_lamports field is not yet implemented")]
    InLamportsUnimplemented, // 6145
    #[msg("out_lamports field is not yet implemented")]
    OutLamportsUnimplemented, // 6146
    #[msg("Mints with restricted extensions require compressible accounts")]
    CompressibleRequired, // 6147
    #[msg("CMint account not found")]
    CMintNotFound, // 6148
    #[msg("CompressedOnly inputs must decompress to CToken account, not SPL token account")]
    CompressedOnlyRequiresCTokenDecompress, // 6149
    #[msg("Invalid token data version")]
    InvalidTokenDataVersion, // 6150
    #[msg("compression_only can only be set for mints with restricted extensions")]
    CompressionOnlyNotAllowed, // 6151
    #[msg("Associated token accounts must have compression_only set")]
    AtaRequiresCompressionOnly, // 6152
    // =========================================================================
    // SPL Token compatible errors (mapped from pinocchio token processor)
    // These mirror SPL Token error codes for consistent error reporting
    // =========================================================================
    #[msg("Lamport balance below rent-exempt threshold")]
    NotRentExempt, // 6153 (SPL Token code 0)
    #[msg("Insufficient funds for the operation")]
    InsufficientFunds, // 6154 (SPL Token code 1)
    #[msg("Account not associated with this Mint")]
    MintMismatch, // 6155 (SPL Token code 3)
    #[msg("This token's supply is fixed and new tokens cannot be minted")]
    FixedSupply, // 6156 (SPL Token code 5)
    #[msg("Account already in use")]
    AlreadyInUse, // 6157 (SPL Token code 6)
    #[msg("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners, // 6158 (SPL Token code 7)
    #[msg("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners, // 6159 (SPL Token code 8)
    #[msg("State is uninitialized")]
    UninitializedState, // 6160 (SPL Token code 9)
    #[msg("Instruction does not support native tokens")]
    NativeNotSupported, // 6161 (SPL Token code 10)
    #[msg("Invalid instruction")]
    InvalidInstruction, // 6162 (SPL Token code 12)
    #[msg("State is invalid for requested operation")]
    InvalidState, // 6163 (SPL Token code 13)
    #[msg("Operation overflowed")]
    Overflow, // 6164 (SPL Token code 14)
    #[msg("Account does not support specified authority type")]
    AuthorityTypeNotSupported, // 6165 (SPL Token code 15)
    #[msg("Mint decimals mismatch between the client and mint")]
    MintDecimalsMismatch, // 6166 (SPL Token code 18)
    #[msg("Failed to calculate rent exemption for CMint")]
    CMintRentExemptionFailed, // 6167
}

/// Anchor error code offset - error codes start at 6000
pub const ERROR_CODE_OFFSET: u32 = 6000;

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(ERROR_CODE_OFFSET + e as u32)
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
