/**
 * Light Protocol Token SDK Constants
 */

import { address, type Address } from '@solana/addresses';

// ============================================================================
// PROGRAM IDS
// ============================================================================

/** Light Token Program ID */
export const LIGHT_TOKEN_PROGRAM_ID: Address = address(
    'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
);

/** Light System Program ID */
export const LIGHT_SYSTEM_PROGRAM_ID: Address = address(
    'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
);

/** Account Compression Program ID */
export const ACCOUNT_COMPRESSION_PROGRAM_ID: Address = address(
    'compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq',
);

/** SPL Token Program ID */
export const SPL_TOKEN_PROGRAM_ID: Address = address(
    'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
);

/** SPL Token 2022 Program ID */
export const SPL_TOKEN_2022_PROGRAM_ID: Address = address(
    'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',
);

/** System Program ID */
export const SYSTEM_PROGRAM_ID: Address = address(
    '11111111111111111111111111111111',
);

// ============================================================================
// KNOWN ACCOUNTS
// ============================================================================

/** CPI Authority - used for cross-program invocations */
export const CPI_AUTHORITY: Address = address(
    'GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy',
);

/** Mint Address Tree - default tree for compressed mint addresses */
export const MINT_ADDRESS_TREE: Address = address(
    'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx',
);

/** Native Mint (wrapped SOL) */
export const NATIVE_MINT: Address = address(
    'So11111111111111111111111111111111111111112',
);

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

/**
 * Instruction discriminators for the Light Token program.
 * Uses SPL-compatible values (3-18) plus custom values (100+).
 */
export const DISCRIMINATOR = {
    /** CToken transfer between decompressed accounts */
    TRANSFER: 3,
    /** Approve delegate on CToken account */
    APPROVE: 4,
    /** Revoke delegate on CToken account */
    REVOKE: 5,
    /** Mint tokens to CToken account */
    MINT_TO: 7,
    /** Burn tokens from CToken account */
    BURN: 8,
    /** Close CToken account */
    CLOSE: 9,
    /** Freeze CToken account */
    FREEZE: 10,
    /** Thaw frozen CToken account */
    THAW: 11,
    /** Transfer with decimals validation */
    TRANSFER_CHECKED: 12,
    /** Mint with decimals validation */
    MINT_TO_CHECKED: 14,
    /** Burn with decimals validation */
    BURN_CHECKED: 15,
    /** Create CToken account */
    CREATE_TOKEN_ACCOUNT: 18,
    /** Create associated CToken account */
    CREATE_ATA: 100,
    /** Batch transfer instruction (compressed/decompressed) */
    TRANSFER2: 101,
    /** Create associated CToken account (idempotent) */
    CREATE_ATA_IDEMPOTENT: 102,
    /** Batch mint action instruction */
    MINT_ACTION: 103,
    /** Claim rent from compressible accounts */
    CLAIM: 104,
    /** Withdraw from funding pool */
    WITHDRAW_FUNDING_POOL: 105,
} as const;

export type Discriminator = (typeof DISCRIMINATOR)[keyof typeof DISCRIMINATOR];

// ============================================================================
// COMPRESSION MODES
// ============================================================================

/**
 * Compression mode for Transfer2 instruction.
 */
export const COMPRESSION_MODE = {
    /** Compress: SPL/CToken -> compressed token */
    COMPRESS: 0,
    /** Decompress: compressed token -> SPL/CToken */
    DECOMPRESS: 1,
    /** Compress and close the source account */
    COMPRESS_AND_CLOSE: 2,
} as const;

export type CompressionMode =
    (typeof COMPRESSION_MODE)[keyof typeof COMPRESSION_MODE];

// ============================================================================
// EXTENSION DISCRIMINANTS
// ============================================================================

/**
 * Extension discriminant values for TLV data.
 */
export const EXTENSION_DISCRIMINANT = {
    /** Token metadata extension */
    TOKEN_METADATA: 19,
    /** CompressedOnly extension */
    COMPRESSED_ONLY: 31,
    /** Compressible extension */
    COMPRESSIBLE: 32,
} as const;

export type ExtensionDiscriminant =
    (typeof EXTENSION_DISCRIMINANT)[keyof typeof EXTENSION_DISCRIMINANT];

// ============================================================================
// SEEDS
// ============================================================================

/** Compressed mint PDA seed */
export const COMPRESSED_MINT_SEED = 'compressed_mint';

/** Pool PDA seed for SPL interface */
export const POOL_SEED = 'pool';

/** Restricted pool PDA seed */
export const RESTRICTED_POOL_SEED = 'restricted';

// ============================================================================
// ACCOUNT SIZES
// ============================================================================

/** Size of a compressed mint account */
export const MINT_ACCOUNT_SIZE = 82n;

/** Base size of a CToken account (without extensions) */
export const BASE_TOKEN_ACCOUNT_SIZE = 266n;

/** Extension metadata overhead (Vec length) */
export const EXTENSION_METADATA_SIZE = 4n;

/** CompressedOnly extension size */
export const COMPRESSED_ONLY_EXTENSION_SIZE = 16n;

/** Transfer fee account extension size */
export const TRANSFER_FEE_ACCOUNT_EXTENSION_SIZE = 9n;

/** Transfer hook account extension size */
export const TRANSFER_HOOK_ACCOUNT_EXTENSION_SIZE = 2n;
