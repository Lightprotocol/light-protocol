// TODO: consider implementing BN254 as wrapper class around _BN mirroring
// PublicKey this would encapsulate our runtime checks and also enforce
// typesafety at compile time
import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { FIELD_SIZE } from '../constants';
import { PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
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
) => new BN(number, base, endian);

/** Create a bigint instance with <254-bit max size and base58 capabilities */
export const createBN254 = (
    number: string | number | BN | Buffer | Uint8Array | number[],
    base?: number | 'hex' | 'base58' | undefined,
): BN254 => {
    if (base === 'base58') {
        if (typeof number !== 'string')
            throw new Error('Must be a base58 string');
        return createBN254(Buffer.from(bs58.decode(number)));
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

/** Convert <254-bit bigint to Base58 string. Fills up to 32 bytes. */
export function encodeBN254toBase58(bigintNumber: BN254, pad = 32): string {
    let buffer = Buffer.from(bigintNumber.toString(16), 'hex');
    // Ensure the buffer is 32 bytes. If not, pad it with leading zeros.
    if (buffer.length < pad) {
        const padding = Buffer.alloc(pad - buffer.length);
        buffer = Buffer.concat([padding, buffer], pad);
    }
    return bs58.encode(buffer);
}
/** Convert Base58 string to <254-bit Solana Public key*/
export function bigint254ToPublicKey(bigintNumber: BN254): PublicKey {
    const paddedBase58 = encodeBN254toBase58(bigintNumber);
    return new PublicKey(paddedBase58);
}

// FIXME: assumes <254 bit pubkey. just use consistent type (pubkey254)
/** Convert Solana Public key to <254-bit bigint */
export function PublicKeyToBN254(publicKey: PublicKey): BN254 {
    const buffer = publicKey.toBuffer();
    // Remove leading zeros from the buffer
    const trimmedBuffer = buffer.subarray(buffer.findIndex(byte => byte !== 0));
    return createBN254(trimmedBuffer);
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
            console.log('bigint', bigint, bigint.toString());
            expect(bigint.toNumber()).toBe(bn(100).toNumber());
        });
    });

    describe('encodeBN254toBase58 function', () => {
        it('should convert a BN254 to a base58 string, no pad', () => {
            const bigint = createBN254('100');
            const base58 = encodeBN254toBase58(bigint, 0);
            expect(base58).toBe('2j');
        });

        it('should convert a BN254 to a base58 string, with pad', () => {
            const bigint = createBN254('100');
            const base58 = encodeBN254toBase58(bigint);
            expect(base58).toBe('11111111111111111111111111111112j');
        });
        it('should throw an error for a value that is too large', () => {
            expect(() => createBN254(FIELD_SIZE)).toThrow(
                'Value is too large. Max <254 bits',
            );
        });
    });

    describe('bigint254ToPublicKey function', () => {
        it('should convert a BN254 to a PublicKey', () => {
            const bigint = createBN254('100');
            const publicKey = bigint254ToPublicKey(bigint);
            expect(publicKey).toBeInstanceOf(PublicKey);
        });
    });

    describe('PublicKeyToBigint254 function', () => {
        it('should convert a PublicKey to a BN254', () => {
            const publicKey = PublicKey.unique();
            const bigint = PublicKeyToBN254(publicKey);
            expect(bigint).toBeInstanceOf(BN);
        });
    });
}
