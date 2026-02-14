/**
 * Type definitions for Light Token codecs
 */

import type { Address } from '@solana/addresses';
import type { ReadonlyUint8Array } from '@solana/codecs';

// ============================================================================
// COMPRESSION TYPES
// ============================================================================

/**
 * Compression operation for Transfer2 instruction.
 * Describes how to compress/decompress tokens.
 */
export interface Compression {
    /** Compression mode: 0=compress, 1=decompress, 2=compress_and_close */
    mode: number;
    /** Amount to compress/decompress */
    amount: bigint;
    /** Index of mint in packed accounts */
    mint: number;
    /** Index of source (compress) or recipient (decompress) in packed accounts */
    sourceOrRecipient: number;
    /** Index of authority in packed accounts */
    authority: number;
    /** Index of pool account in packed accounts */
    poolAccountIndex: number;
    /** Pool index (for multi-pool mints) */
    poolIndex: number;
    /** PDA bump for pool derivation */
    bump: number;
    /** Token decimals (or rent_sponsor_is_signer flag for CompressAndClose) */
    decimals: number;
}

// ============================================================================
// MERKLE CONTEXT TYPES
// ============================================================================

/**
 * Packed merkle context for compressed accounts.
 */
export interface PackedMerkleContext {
    /** Index of merkle tree pubkey in packed accounts */
    merkleTreePubkeyIndex: number;
    /** Index of queue pubkey in packed accounts */
    queuePubkeyIndex: number;
    /** Leaf index in the merkle tree */
    leafIndex: number;
    /** Whether to prove by index (vs by hash) */
    proveByIndex: boolean;
}

// ============================================================================
// TOKEN DATA TYPES
// ============================================================================

/**
 * Input token data with merkle context for Transfer2.
 */
export interface MultiInputTokenDataWithContext {
    /** Index of owner in packed accounts */
    owner: number;
    /** Token amount */
    amount: bigint;
    /** Whether token has a delegate */
    hasDelegate: boolean;
    /** Index of delegate in packed accounts (if hasDelegate) */
    delegate: number;
    /** Index of mint in packed accounts */
    mint: number;
    /** Token account version */
    version: number;
    /** Merkle context for the compressed account */
    merkleContext: PackedMerkleContext;
    /** Root index for validity proof */
    rootIndex: number;
}

/**
 * Output token data for Transfer2.
 */
export interface MultiTokenTransferOutputData {
    /** Index of owner in packed accounts */
    owner: number;
    /** Token amount */
    amount: bigint;
    /** Whether token has a delegate */
    hasDelegate: boolean;
    /** Index of delegate in packed accounts (if hasDelegate) */
    delegate: number;
    /** Index of mint in packed accounts */
    mint: number;
    /** Token account version */
    version: number;
}

// ============================================================================
// CPI CONTEXT
// ============================================================================

/**
 * CPI context for compressed account operations.
 */
export interface CompressedCpiContext {
    /** Whether to set the CPI context */
    setContext: boolean;
    /** Whether this is the first set context call */
    firstSetContext: boolean;
}

// ============================================================================
// PROOF TYPES
// ============================================================================

/**
 * Groth16 proof for compressed account validity.
 */
export interface CompressedProof {
    /** Proof element A (32 bytes) */
    a: ReadonlyUint8Array;
    /** Proof element B (64 bytes) */
    b: ReadonlyUint8Array;
    /** Proof element C (32 bytes) */
    c: ReadonlyUint8Array;
}

// ============================================================================
// EXTENSION TYPES
// ============================================================================

/**
 * Token metadata extension data.
 */
export interface TokenMetadataExtension {
    /** Update authority (optional) */
    updateAuthority: Address | null;
    /** Token name */
    name: ReadonlyUint8Array;
    /** Token symbol */
    symbol: ReadonlyUint8Array;
    /** Token URI */
    uri: ReadonlyUint8Array;
    /** Additional metadata key-value pairs */
    additionalMetadata: Array<{
        key: ReadonlyUint8Array;
        value: ReadonlyUint8Array;
    }> | null;
}

/**
 * CompressedOnly extension data.
 */
export interface CompressedOnlyExtension {
    /** Delegated amount */
    delegatedAmount: bigint;
    /** Withheld transfer fee */
    withheldTransferFee: bigint;
    /** Whether account is frozen */
    isFrozen: boolean;
    /** Compression index */
    compressionIndex: number;
    /** Whether this is an ATA */
    isAta: boolean;
    /** PDA bump */
    bump: number;
    /** Owner index in packed accounts */
    ownerIndex: number;
}

/**
 * Rent configuration for compressible accounts.
 */
export interface RentConfig {
    /** Base rent in lamports */
    baseRent: number;
    /** Compression cost in lamports */
    compressionCost: number;
    /** Lamports per byte per epoch */
    lamportsPerBytePerEpoch: number;
    /** Maximum funded epochs */
    maxFundedEpochs: number;
    /** Maximum top-up amount */
    maxTopUp: number;
}

/**
 * Compression info for compressible accounts.
 */
export interface CompressionInfo {
    /** Config account version */
    configAccountVersion: number;
    /** Compress-to pubkey type: 0=none, 1=owner, 2=custom */
    compressToPubkey: number;
    /** Account version */
    accountVersion: number;
    /** Lamports per write operation */
    lamportsPerWrite: number;
    /** Compression authority (32 bytes) */
    compressionAuthority: Uint8Array;
    /** Rent sponsor (32 bytes) */
    rentSponsor: Uint8Array;
    /** Last claimed slot */
    lastClaimedSlot: bigint;
    /** Rent exemption paid */
    rentExemptionPaid: number;
    /** Reserved bytes */
    reserved: number;
    /** Rent configuration */
    rentConfig: RentConfig;
}

/**
 * Transfer fee account extension data.
 */
export interface TransferFeeAccountExtension {
    /** Withheld transfer fee amount */
    withheldAmount: bigint;
}

/**
 * Transfer hook account extension data.
 */
export interface TransferHookAccountExtension {
    /** Reentrancy guard (always 0 at rest in Light Protocol) */
    transferring: number;
}

/**
 * Extension instruction data (union type).
 */
export type ExtensionInstructionData =
    | { type: 'TokenMetadata'; data: TokenMetadataExtension }
    | { type: 'PausableAccount' }
    | { type: 'PermanentDelegateAccount' }
    | { type: 'TransferFeeAccount' }
    | { type: 'TransferHookAccount' }
    | { type: 'CompressedOnly'; data: CompressedOnlyExtension }
    | { type: 'Compressible'; data: CompressionInfo };

// ============================================================================
// TRANSFER2 INSTRUCTION DATA
// ============================================================================

/**
 * Full Transfer2 instruction data.
 */
export interface Transfer2InstructionData {
    /** Whether to include transaction hash in hashing */
    withTransactionHash: boolean;
    /** Whether to include lamports change account merkle tree index */
    withLamportsChangeAccountMerkleTreeIndex: boolean;
    /** Merkle tree index for lamports change account */
    lamportsChangeAccountMerkleTreeIndex: number;
    /** Owner index for lamports change account */
    lamportsChangeAccountOwnerIndex: number;
    /** Output queue index */
    outputQueue: number;
    /** Maximum top-up for rent */
    maxTopUp: number;
    /** CPI context (optional) */
    cpiContext: CompressedCpiContext | null;
    /** Compression operations (optional) */
    compressions: Compression[] | null;
    /** Validity proof (optional) */
    proof: CompressedProof | null;
    /** Input token data */
    inTokenData: MultiInputTokenDataWithContext[];
    /** Output token data */
    outTokenData: MultiTokenTransferOutputData[];
    /** Input lamports (optional) */
    inLamports: bigint[] | null;
    /** Output lamports (optional) */
    outLamports: bigint[] | null;
    /** Input TLV extensions (optional) */
    inTlv: ExtensionInstructionData[][] | null;
    /** Output TLV extensions (optional) */
    outTlv: ExtensionInstructionData[][] | null;
}

// ============================================================================
// COMPRESSIBLE CONFIG TYPES
// ============================================================================

/**
 * Compress-to pubkey configuration.
 */
export interface CompressToPubkey {
    /** PDA bump */
    bump: number;
    /** Program ID for the PDA */
    programId: ReadonlyUint8Array;
    /** Seeds for the PDA */
    seeds: ReadonlyUint8Array[];
}

/**
 * Compressible extension instruction data for create instructions.
 */
export interface CompressibleExtensionInstructionData {
    /** Token account version */
    tokenAccountVersion: number;
    /** Number of epochs to pre-pay rent */
    rentPayment: number;
    /** Compression only mode: 0=false, 1=true */
    compressionOnly: number;
    /** Lamports per write for top-up */
    writeTopUp: number;
    /** Compress-to pubkey configuration (optional) */
    compressToPubkey: CompressToPubkey | null;
}

// ============================================================================
// CREATE ACCOUNT TYPES
// ============================================================================

/**
 * Create Associated Token Account instruction data.
 * Note: bump is NOT included in instruction data — the on-chain program
 * derives it via validate_ata_derivation.
 */
export interface CreateAtaInstructionData {
    /** Compressible config (optional) */
    compressibleConfig: CompressibleExtensionInstructionData | null;
}

/**
 * Create Token Account instruction data.
 */
export interface CreateTokenAccountInstructionData {
    /** Owner of the token account */
    owner: Address;
    /** Compressible config (optional) */
    compressibleConfig: CompressibleExtensionInstructionData | null;
}
