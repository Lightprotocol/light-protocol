// TODO: consider implementing BN254 as wrapper class around _BN mirroring
// PublicKey this would encapsulate our runtime checks and also enforce
// typesafety at compile time

import { FIELD_SIZE } from '../constants';
import { PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { bs58 } from '@coral-xyz/anchor/dist/esm/utils/bytes';
import { Buffer } from 'buffer';

/**
 * bignumber with <254-bit max size. Anchor serialization doesn't support native
 * bigint yet, so we wrap BN. This wrapper has simple base10 encoding which is
 * needed for zk circuit compat, in addition to the base58 encoding that users
 * are used to from working with the web3.js PublicKey type.
 */
export type BN254 = BN;

export const bn = (
    number: string | number | BN | Buffer | Uint8Array | number[],
    base?: number | 'hex' | undefined,
    endian?: BN.Endianness | undefined,
): BN => new BN(number, base, endian);

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

    return bs58.encode(bn254Buffer);
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    describe('createBN254 function', () => {
        it('should create a BN254 from a string', () => {
            const bigint = createBN254('100');
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a number', () => {
            const bigint = createBN254(100);
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a bigint', () => {
            const bigint = createBN254(bn(100));
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a Buffer', () => {
            const bigint = createBN254(Buffer.from([100]));
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a Uint8Array', () => {
            const bigint = createBN254(new Uint8Array([100]));
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a number[]', () => {
            const bigint = createBN254([100]);
            expect(bigint.toNumber()).toBe(100);
        });

        it('should create a BN254 from a base58 string', () => {
            const bigint = createBN254('2j', 'base58');
            expect(bigint.toNumber()).toBe(bn(100).toNumber());
        });
    });

    describe('encodeBN254toBase58 function', () => {
        it('should convert a BN254 to a base58 string, pad to 32 implicitly', () => {
            const bigint = createBN254('100');
            const base58 = encodeBN254toBase58(bigint);
            expect(base58).toBe('11111111111111111111111111111112j');
        });

        it('should match transformation via pubkey', () => {
            const refHash = [
                13, 225, 248, 105, 237, 121, 108, 70, 70, 197, 240, 130, 226,
                236, 129, 58, 213, 50, 236, 99, 216, 99, 91, 201, 141, 76, 196,
                33, 41, 181, 236, 187,
            ];
            const base58 = encodeBN254toBase58(bn(refHash));

            const pubkeyConv = new PublicKey(refHash).toBase58();
            expect(base58).toBe(pubkeyConv);
        });

        it('should pad to 32 bytes converting BN to Pubkey', () => {
            const refHash31 = [
                13, 225, 248, 105, 237, 121, 108, 70, 70, 197, 240, 130, 226,
                236, 129, 58, 213, 50, 236, 99, 216, 99, 91, 201, 141, 76, 196,
                33, 41, 181, 236,
            ];
            const base58 = encodeBN254toBase58(bn(refHash31));

            expect(
                createBN254(base58, 'base58').toArray('be', 32),
            ).to.be.deep.equal([0].concat(refHash31));
        });

        it('should throw an error for a value that is too large', () => {
            expect(() => createBN254(FIELD_SIZE)).toThrow(
                'Value is too large. Max <254 bits',
            );
        });
    });
}
