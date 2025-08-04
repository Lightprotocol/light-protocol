import { PublicKey, AccountMeta } from '@solana/web3.js';
import BN from 'bn.js';
import { ValidityProof } from '../state/types';
import { CompressedAccountMeta } from '../state/compressed-account';

/**
 * Standard instruction discriminators for compressible instructions
 * These match the Rust implementation discriminators
 */
export const COMPRESSIBLE_DISCRIMINATORS = {
    INITIALIZE_COMPRESSION_CONFIG: [133, 228, 12, 169, 56, 76, 222, 61],
    UPDATE_COMPRESSION_CONFIG: [135, 215, 243, 81, 163, 146, 33, 70],
    DECOMPRESS_ACCOUNTS_IDEMPOTENT: [114, 67, 61, 123, 234, 31, 1, 112],
} as const;

/**
 * Generic compressed account data structure for decompress operations
 * This is generic over the account variant type, allowing programs to use their specific enums
 */
export type CompressedAccountData<T = any> = {
    /** The compressed account metadata containing tree info, address, and output index */
    meta: CompressedAccountMeta;
    /** Program-specific account variant enum */
    data: T;
    /** PDA seeds (without bump) used to derive the PDA address */
    seeds: Uint8Array[];
};

/**
 * Instruction data structure for decompress_accounts_idempotent
 * This matches the exact format expected by Anchor programs
 */
export type DecompressMultipleAccountsIdempotentData<T = any> = {
    proof: ValidityProof;
    compressedAccounts: CompressedAccountData<T>[];
    bumps: number[];
    systemAccountsOffset: number;
};

/**
 * Instruction data for update compression config
 */
export type UpdateCompressionConfigData = {
    newCompressionDelay: number | null;
    newRentRecipient: PublicKey | null;
    newAddressSpace: PublicKey[] | null;
    newUpdateAuthority: PublicKey | null;
};

/**
 * Generic instruction data for compress account
 * This matches the expected format for compress account instructions
 */
export type GenericCompressAccountInstruction = {
    proof: ValidityProof;
    compressedAccountMeta: CompressedAccountMeta;
};

/**
 * Existing CompressionConfigIxData type (re-exported for compatibility)
 */
export type CompressionConfigIxData = {
    compressionDelay: number;
    rentRecipient: PublicKey;
    addressSpace: PublicKey[];
    configBump: number | null;
};

/**
 * Common instruction builder parameters
 */
export type InstructionBuilderParams = {
    programId: PublicKey;
    discriminator: Uint8Array | number[];
};

/**
 * Initialize compression config instruction parameters
 */
export type InitializeCompressionConfigParams = InstructionBuilderParams & {
    payer: PublicKey;
    authority: PublicKey;
    compressionDelay: number;
    rentRecipient: PublicKey;
    addressSpace: PublicKey[];
    configBump?: number | null;
};

/**
 * Update compression config instruction parameters
 */
export type UpdateCompressionConfigParams = InstructionBuilderParams & {
    authority: PublicKey;
    newCompressionDelay?: number | null;
    newRentRecipient?: PublicKey | null;
    newAddressSpace?: PublicKey[] | null;
    newUpdateAuthority?: PublicKey | null;
};

/**
 * Compress account instruction parameters
 */
export type CompressAccountParams = InstructionBuilderParams & {
    payer: PublicKey;
    pdaToCompress: PublicKey;
    rentRecipient: PublicKey;
    compressedAccountMeta: CompressedAccountMeta;
    validityProof: ValidityProof;
    systemAccounts?: AccountMeta[];
};

/**
 * Decompress accounts idempotent instruction parameters
 */
export type DecompressAccountsIdempotentParams<T = any> =
    InstructionBuilderParams & {
        feePayer: PublicKey;
        rentPayer: PublicKey;
        solanaAccounts: PublicKey[];
        compressedAccountsData: CompressedAccountData<T>[];
        bumps: number[];
        validityProof: ValidityProof;
        systemAccounts?: AccountMeta[];
        dataSchema?: any; // borsh.Layout<T> - keeping it flexible
    };
