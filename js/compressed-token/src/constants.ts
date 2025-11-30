import { Buffer } from 'buffer';

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
