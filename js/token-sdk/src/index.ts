/**
 * Light Protocol Token SDK
 *
 * TypeScript SDK for Light Protocol compressed tokens using Solana Kit (web3.js v2).
 *
 * @example
 * ```typescript
 * import {
 *   createTransferInstruction,
 *   createAssociatedTokenAccountInstruction,
 *   deriveAssociatedTokenAddress,
 *   LIGHT_TOKEN_PROGRAM_ID,
 * } from '@lightprotocol/token-sdk';
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
    MINT_ADDRESS_TREE,
    NATIVE_MINT,

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

    // Validation
    isLightTokenAccount,
    determineTransferType,
    validateAtaDerivation,
    validatePositiveAmount,
    validateDecimals,
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
    getCompressToPubkeyCodec,
    getCompressibleExtensionDataCodec,
    getCreateAtaDataCodec,
    encodeCreateAtaInstructionData,
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
    type TransferType,
    type TransferInterfaceParams,
    type TransferInterfaceResult,

    // Account
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
    createCloseAccountInstruction,
    type CreateAtaParams,
    type CreateAtaResult,
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

    // Mint
    createMintToInstruction,
    createMintToCheckedInstruction,
    type MintToParams,
    type MintToCheckedParams,
} from './instructions/index.js';

// ============================================================================
// CLIENT TYPES (Indexer & load functions in @lightprotocol/token-client)
// ============================================================================

export {
    // Validation
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
} from './client/index.js';
