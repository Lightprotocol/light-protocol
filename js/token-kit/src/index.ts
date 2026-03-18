/**
 * Light Protocol Token Kit
 *
 * Unified TypeScript SDK for Light Protocol compressed tokens using Solana Kit (web3.js v2).
 * Includes instructions, codecs, indexer client, account loading, and high-level actions.
 *
 * @example
 * ```typescript
 * import {
 *   createTransferInstruction,
 *   createAssociatedTokenAccountInstruction,
 *   deriveAssociatedTokenAddress,
 *   LIGHT_TOKEN_PROGRAM_ID,
 *   createLightIndexer,
 *   loadTokenAccountsForTransfer,
 *   buildCompressedTransfer,
 * } from '@lightprotocol/token-kit';
 *
 * // Derive ATA address
 * const { address: ata, bump } = await deriveAssociatedTokenAddress(owner, mint);
 *
 * // Create transfer instruction
 * const transferIx = createTransferInstruction({
 *   source: sourceAta,
 *   destination: destAta,
 *   amount: 1000n,
 *   authority: owner,
 * });
 *
 * // Or use the high-level builder
 * const indexer = createLightIndexer('https://photon.helius.dev');
 * const result = await buildCompressedTransfer(indexer, {
 *   owner, mint, amount: 1000n, recipientOwner, feePayer,
 * });
 * ```
 *
 * @packageDocumentation
 */

// ============================================================================
// CONSTANTS
// ============================================================================

export {
    // Program IDs
    LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID,
    ACCOUNT_COMPRESSION_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID,
    SPL_TOKEN_2022_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,

    // Known accounts
    CPI_AUTHORITY,
    REGISTERED_PROGRAM_PDA,
    ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    MINT_ADDRESS_TREE,
    NATIVE_MINT,
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
    NOOP_PROGRAM,

    // Instruction discriminators
    DISCRIMINATOR,
    type Discriminator,

    // Compression modes
    COMPRESSION_MODE,
    type CompressionMode,

    // Extension discriminants
    EXTENSION_DISCRIMINANT,
    type ExtensionDiscriminant,

    // Seeds
    COMPRESSED_MINT_SEED,
    POOL_SEED,
    RESTRICTED_POOL_SEED,

    // Account sizes
    MINT_ACCOUNT_SIZE,
    BASE_TOKEN_ACCOUNT_SIZE,
    EXTENSION_METADATA_SIZE,
    COMPRESSED_ONLY_EXTENSION_SIZE,
    TRANSFER_FEE_ACCOUNT_EXTENSION_SIZE,
    TRANSFER_HOOK_ACCOUNT_EXTENSION_SIZE,
    TOKEN_ACCOUNT_VERSION_V2,
} from './constants.js';

// ============================================================================
// UTILITIES
// ============================================================================

export {
    // PDA derivation
    deriveAssociatedTokenAddress,
    getAssociatedTokenAddressWithBump,
    deriveMintAddress,
    derivePoolAddress,
    deriveCompressedAddress,
    deriveCompressedMintAddress,

    // Validation
    isLightTokenAccount,
    determineTransferType,
    type TransferType,
    validateAtaDerivation,
    validatePositiveAmount,
    validateDecimals,

    // SPL interface
    type SplInterfaceInfo,
    getSplInterfaceInfo,
    getSplInterfaceInfos,
    selectSplInterfaceInfo,
    selectSplInterfaceInfosForDecompression,
    deriveSplInterfaceInfo,
} from './utils/index.js';

// ============================================================================
// CODECS
// ============================================================================

export {
    // Types
    type Compression,
    type PackedMerkleContext,
    type MultiInputTokenDataWithContext,
    type MultiTokenTransferOutputData,
    type CompressedCpiContext,
    type CompressedProof,
    type TokenMetadataExtension,
    type CompressedOnlyExtension,
    type RentConfig,
    type CompressionInfo,
    type ExtensionInstructionData,
    type Transfer2InstructionData,
    type CompressToPubkey,
    type CompressibleExtensionInstructionData,
    type CreateAtaInstructionData,
    type CreateTokenAccountInstructionData,

    // Mint deserializer
    deserializeCompressedMint,
    type BaseMint,
    type DeserializedMintContext,
    type DeserializedCompressedMint,

    // Transfer2 codecs
    getCompressionCodec,
    getPackedMerkleContextCodec,
    getMultiInputTokenDataCodec,
    getMultiTokenOutputDataCodec,
    getCpiContextCodec,
    getCompressedProofCodec,
    encodeTransfer2InstructionData,
    type Transfer2BaseInstructionData,

    // Compressible codecs
    getCompressibleExtensionDataCodec,
    getCreateAtaDataCodec,
    getCreateTokenAccountDataCodec,
    encodeCreateAtaInstructionData,
    encodeCreateTokenAccountInstructionData,
    defaultCompressibleParams,

    // Simple instruction codecs
    getAmountInstructionCodec,
    getCheckedInstructionCodec,
    getDiscriminatorOnlyCodec,
    encodeMaxTopUp,
    decodeMaxTopUp,
    type AmountInstructionData,
    type CheckedInstructionData,
    type DiscriminatorOnlyData,

    // MintAction codecs
    encodeMintActionInstructionData,
    type MintRecipient,
    type MintToCompressedAction,
    type MintToAction,
    type UpdateAuthorityAction,
    type UpdateMetadataFieldAction,
    type UpdateMetadataAuthorityAction,
    type RemoveMetadataKeyAction,
    type DecompressMintAction,
    type CompressAndCloseMintAction,
    type MintAction,
    type CreateMint,
    type MintMetadata,
    type MintInstructionData,
    type MintActionCpiContext,
    type MintActionInstructionData,
} from './codecs/index.js';

// ============================================================================
// INSTRUCTIONS
// ============================================================================

export {
    // Transfer
    createTransferInstruction,
    createTransferCheckedInstruction,
    createTransferInterfaceInstruction,
    requiresCompression,
    type TransferParams,
    type TransferCheckedParams,
    type TransferInterfaceParams,
    type TransferInterfaceResult,

    // Account
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
    createTokenAccountInstruction,
    createCloseAccountInstruction,
    type CreateAtaParams,
    type CreateAtaResult,
    type CreateTokenAccountParams,
    type CloseAccountParams,

    // Token operations
    createApproveInstruction,
    createRevokeInstruction,
    createBurnInstruction,
    createBurnCheckedInstruction,
    createFreezeInstruction,
    createThawInstruction,
    type ApproveParams,
    type RevokeParams,
    type BurnParams,
    type BurnCheckedParams,
    type FreezeParams,
    type ThawParams,
    type FreezeThawParams,

    // Mint
    createMintToInstruction,
    createMintToCheckedInstruction,
    type MintToParams,
    type MintToCheckedParams,

    // Transfer2 (compressed account operations)
    createTransfer2Instruction,
    type Transfer2Params,

    // Compression factory functions (for Transfer2)
    createCompress,
    createCompressSpl,
    createDecompress,
    createDecompressSpl,
    createCompressAndClose,

    // MintAction (compressed mint management)
    createMintActionInstruction,
    type MintActionParams,
    type MintActionCpiContextAccounts,

    // Rent management
    createClaimInstruction,
    type ClaimParams,
    createWithdrawFundingPoolInstruction,
    type WithdrawFundingPoolParams,

    // Wrap/Unwrap (SPL ↔ Light Token)
    createWrapInstruction,
    createUnwrapInstruction,
    type WrapParams,
    type UnwrapParams,

    // SPL interface PDA
    createSplInterfaceInstruction,
    addSplInterfacesInstruction,
    type CreateSplInterfaceParams,
    type CreateSplInterfaceResult,
    type AddSplInterfacesParams,
} from './instructions/index.js';

// ============================================================================
// CLIENT TYPES
// ============================================================================

export {
    // Validation
    assertValidTreeType,
    assertV2Tree,

    // Types
    TreeType,
    AccountState,
    IndexerErrorCode,
    IndexerError,
    type TreeInfo,
    type CompressedAccountData,
    type CompressedAccount,
    type TokenData,
    type CompressedTokenAccount,
    type ValidityProof,
    type RootIndex,
    type AccountProofInputs,
    type AddressProofInputs,
    type ValidityProofWithContext,
    type AddressWithTree,
    type GetCompressedTokenAccountsOptions,
    type ResponseContext,
    type IndexerResponse,
    type ItemsWithCursor,
    type TokenBalance,
    type TokenHolder,
    type SignatureInfo,
} from './client/index.js';

// ============================================================================
// INDEXER
// ============================================================================

export {
    type LightIndexer,
    PhotonIndexer,
    createLightIndexer,
    isLightIndexerAvailable,
} from './indexer.js';

// ============================================================================
// LOAD FUNCTIONS
// ============================================================================

export {
    // Types
    type InputTokenAccount,
    type MerkleContext,
    type LoadedTokenAccounts,
    type LoadTokenAccountsOptions,
    type SelectedAccounts,
    type MintContext,

    // Load functions
    loadTokenAccountsForTransfer,
    loadTokenAccount,
    loadAllTokenAccounts,
    loadCompressedAccount,
    loadCompressedAccountByHash,
    loadMintContext,

    // Account selection
    selectAccountsForAmount,
    DEFAULT_MAX_INPUTS,

    // Proof helpers
    getValidityProofForAccounts,
    needsValidityProof,
    getTreeInfo,
    getOutputTreeInfo,
} from './load.js';

// ============================================================================
// ACTIONS (high-level builders)
// ============================================================================

export {
    // Transfer
    buildCompressedTransfer,
    buildTransferDelegated,
    buildTransferInterface,

    // Compress / Decompress
    buildCompress,
    buildDecompress,
    buildCompressSplTokenAccount,
    buildDecompressInterface,

    // Wrap / Unwrap
    buildWrap,
    buildUnwrap,

    // Mint management
    buildCreateMint,
    buildUpdateMintAuthority,
    buildUpdateFreezeAuthority,
    buildUpdateMetadataField,
    buildUpdateMetadataAuthority,
    buildRemoveMetadataKey,
    buildDecompressMint,

    // Mint to
    buildMintToCompressed,
    buildMintToInterface,
    buildApproveAndMintTo,

    // ATA
    buildCreateAta,
    buildCreateAtaIdempotent,
    buildGetOrCreateAta,

    // Load
    buildLoadAta,

    // Types
    type BuildTransferResult,
    type BuilderRpc,
    type MetadataFieldType,
    type MintRecipientParam,
} from './actions.js';

// ============================================================================
// QUERIES
// ============================================================================

export {
    getAtaInterface,
    getMintInterface,
    getMintDecimals,
    type QueryRpc,
    type AtaInterface,
    type MintInterface,
    type TokenAccountSource,
} from './queries.js';
