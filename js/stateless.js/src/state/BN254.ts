import { FIELD_SIZE } from '../constants';
import BN from 'bn.js';
import bs58 from 'bs58';
import { Buffer } from 'buffer';

/**
 * bignumber with <254-bit max size. Anchor serialization doesn't support native
 * bigint yet, so we wrap BN. This wrapper has simple base10 encoding which is
 * needed for zk circuit compat, in addition to the base58 encoding that users
 * are used to from working with the web3.js PublicKey type.
 */
export type BN254 = BN;

/** Create a bigint instance with <254-bit max size and base58 capabilities */
export const createBN254 = (
    number: string | number | BN | Buffer | Uint8Array | number[],
    base?: number | 'hex' | 'base58' | undefined,
): BN254 => {
    if (base === 'base58') {
        if (typeof number !== 'string')
            throw new Error('Must be a base58 string');
        return createBN254(bs58.decode(number));
    }

    const bigintNumber = new BN(number, base);

    return enforceSize(bigintNumber);
};

/**
 * Enforces a maximum size of <254 bits for bigint instances. This is necessary
 * for compatibility with zk-SNARKs, where hashes must be less than the field
 * modulus (~2^254).
 */
function enforceSize(bigintNumber: BN254): BN254 {
    if (bigintNumber.gte(FIELD_SIZE)) {
        throw new Error('Value is too large. Max <254 bits');
    }
    return bigintNumber;
}

/** Convert <254-bit bigint to Base58 string.  */
export function encodeBN254toBase58(bigintNumber: BN): string {
    /// enforce size
    const bn254 = createBN254(bigintNumber);
    const bn254Buffer = bn254.toArrayLike(Buffer, undefined, 32);

    return bs58.encode(new Uint8Array(bn254Buffer));
}
