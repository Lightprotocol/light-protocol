import { Buffer } from 'buffer';
import { PublicKey } from '@solana/web3.js';

/** Default compressible config PDA (V1) */
export const LIGHT_TOKEN_CONFIG = new PublicKey(
    'ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg',
);

/** Default rent sponsor PDA (V1) */
export const LIGHT_TOKEN_RENT_SPONSOR = new PublicKey(
    'r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti',
);

/**
 * Token data version enum - mirrors Rust TokenDataVersion
 * Used for compressed token account hashing strategy
 */
export enum TokenDataVersion {
    /** V1: Poseidon hash with little-endian amount, discriminator [2,0,0,0,0,0,0,0] */
    V1 = 1,
    /** V2: Poseidon hash with big-endian amount, discriminator [0,0,0,0,0,0,0,3] */
    V2 = 2,
    /** ShaFlat: SHA256 hash of borsh-serialized data, discriminator [0,0,0,0,0,0,0,4] */
    ShaFlat = 3,
}

export const POOL_SEED = Buffer.from('pool');

export const CPI_AUTHORITY_SEED = Buffer.from('cpi_authority');

export const SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE = 1461600;

export const CREATE_TOKEN_POOL_DISCRIMINATOR = Buffer.from([
    23, 169, 27, 122, 147, 169, 209, 152,
]);
export const MINT_TO_DISCRIMINATOR = Buffer.from([
    241, 34, 48, 186, 37, 179, 123, 192,
]);
export const BATCH_COMPRESS_DISCRIMINATOR = Buffer.from([
    65, 206, 101, 37, 147, 42, 221, 144,
]);
export const TRANSFER_DISCRIMINATOR = Buffer.from([
    163, 52, 200, 231, 140, 3, 69, 186,
]);
export const COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR = Buffer.from([
    112, 230, 105, 101, 145, 202, 157, 97,
]);

export const APPROVE_DISCRIMINATOR = Buffer.from([
    69, 74, 217, 36, 115, 117, 97, 76,
]);
export const REVOKE_DISCRIMINATOR = Buffer.from([
    170, 23, 31, 34, 133, 173, 93, 242,
]);
export const ADD_TOKEN_POOL_DISCRIMINATOR = Buffer.from([
    114, 143, 210, 73, 96, 115, 1, 228,
]);

export const DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR = Buffer.from([107]);

/**
 * Maximum lamports for rent top-up in a single instruction.
 * u16::MAX = no limit; 0 = no top-ups allowed.
 * Matches Rust SDK (e.g. token-sdk create_mints uses u16::MAX for "no limit").
 */
export const MAX_TOP_UP = 65535;

/**
 * Rent configuration constants for compressible ctoken accounts.
 * These match the Rust SDK defaults in program-libs/compressible/src/rent/config.rs
 */

/** Base rent per epoch (lamports) */
export const BASE_RENT_PER_EPOCH = 128;

/** Rent per byte per epoch (lamports) */
export const RENT_PER_BYTE_PER_EPOCH = 1;

/** Slots per rent epoch (1.5 hours) */
export const SLOTS_PER_RENT_EPOCH = 13500;

/** Compression cost (lamports) - paid at account creation */
export const COMPRESSION_COST = 10000;

/** Compression incentive (lamports) - paid at account creation */
export const COMPRESSION_INCENTIVE = 1000;

/** Total compression cost (COMPRESSION_COST + COMPRESSION_INCENTIVE) */
export const TOTAL_COMPRESSION_COST = COMPRESSION_COST + COMPRESSION_INCENTIVE;

/**
 * Compressible ctoken account size in bytes.
 * = 165 (base SPL token) + 1 (account_type) + 1 (Option) + 4 (Vec len) + 1 (ext disc) + 4 (ext header) + 96 (CompressionInfo) = 272
 * Source: program-libs/token-interface/src/state/token/top_up.rs MIN_SIZE_WITH_COMPRESSIBLE
 */
export const COMPRESSIBLE_CTOKEN_ACCOUNT_SIZE = 272;

/**
 * Calculate rent per epoch for a given account size.
 * Formula: base_rent + (bytes * lamports_per_byte_per_epoch)
 */
export function rentPerEpoch(bytes: number): number {
    return BASE_RENT_PER_EPOCH + bytes * RENT_PER_BYTE_PER_EPOCH;
}

/**
 * Default rent per epoch for a compressible ctoken account (272 bytes).
 * = 128 + 272 = 400 lamports
 */
export const COMPRESSIBLE_CTOKEN_RENT_PER_EPOCH = rentPerEpoch(
    COMPRESSIBLE_CTOKEN_ACCOUNT_SIZE,
);

/** Default prepaid epochs (24 hours = 16 epochs * 1.5h) */
export const DEFAULT_PREPAY_EPOCHS = 16;

/** Default write top-up (lamports) - ~2 epochs rent */
export const DEFAULT_WRITE_TOP_UP = 766;

/**
 * Calculate fee payer cost at ATA creation.
 * = compression_cost (11K) + (prepay_epochs * rent_per_epoch)
 */
export function calculateFeePayerCostAtCreation(
    prepayEpochs: number = DEFAULT_PREPAY_EPOCHS,
    accountBytes: number = COMPRESSIBLE_CTOKEN_ACCOUNT_SIZE,
): number {
    return TOTAL_COMPRESSION_COST + prepayEpochs * rentPerEpoch(accountBytes);
}
