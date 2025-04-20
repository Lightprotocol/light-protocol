import { Buffer } from 'buffer';
import { bn, createBN254 } from '../state';
import { FIELD_SIZE } from '../constants';
import { keccak_256 } from '@noble/hashes/sha3';
import { Keypair } from '@solana/web3.js';
import BN from 'bn.js';
import camelcaseKeys from 'camelcase-keys';

export function byteArrayToKeypair(byteArray: number[]): Keypair {
    return Keypair.fromSecretKey(Uint8Array.from(byteArray));
}
/**
 * @internal
 * convert BN to hex with '0x' prefix
 */
export function toHex(bn: BN): string {
    return '0x' + bn.toString('hex');
}

export const toArray = <T>(value: T | T[]) =>
    Array.isArray(value) ? value : [value];

export const bufToDecStr = (buf: Buffer): string => {
    return createBN254(buf).toString();
};
export function isSmallerThanBn254FieldSizeBe(bytes: Buffer): boolean {
    const bigint = bn(bytes, undefined, 'be');
    return bigint.lt(FIELD_SIZE);
}

export const toCamelCase = (object: any) =>
    camelcaseKeys(object, { deep: true });

/**
/**
 * Hash the provided `bytes` with Keccak256 and ensure the result fits in the
 * BN254 prime field by repeatedly hashing the inputs with various "bump seeds"
 * and truncating the resulting hash to 31 bytes.
 *
 * @deprecated Use `hashvToBn254FieldSizeBe` instead.
 */
export function hashToBn254FieldSizeBe(bytes: Buffer): [Buffer, number] | null {
    // TODO(vadorovsky, affects-onchain): Get rid of the bump mechanism, it
    // makes no sense. Doing the same as in the `hashvToBn254FieldSizeBe` below
    // - overwriting the most significant byte with zero - is sufficient for
    // truncation, it's also faster, doesn't force us to return `Option` and
    // care about handling an error which is practically never returned.
    //
    // The reason we can't do it now is that it would affect on-chain programs.
    // Once we can update programs, we can get rid of the seed bump (or even of
    // this function all together in favor of the `hashv` variant).
    let bumpSeed = 255;
    while (bumpSeed >= 0) {
        const inputWithBumpSeed = Buffer.concat([
            bytes,
            Buffer.from([bumpSeed]),
        ]);
        const hash = keccak_256(inputWithBumpSeed);
        if (hash.length !== 32) {
            throw new Error('Invalid hash length');
        }
        hash[0] = 0;

        if (isSmallerThanBn254FieldSizeBe(Buffer.from(hash))) {
            return [Buffer.from(hash), bumpSeed];
        }

        bumpSeed -= 1;
    }
    return null;
}

/**
 * Hash the provided `bytes` with Keccak256 and ensure that the result fits in
 * the BN254 prime field by truncating the resulting hash to 31 bytes.
 *
 * @param bytes Input bytes
 *
 * @returns     Hash digest
 */
export function hashvToBn254FieldSizeBe(bytes: Uint8Array[]): Uint8Array {
    const hasher = keccak_256.create();
    for (const input of bytes) {
        hasher.update(input);
    }
    const hash = hasher.digest();
    hash[0] = 0;
    return hash;
}

/** Mutates array in place */
export function pushUniqueItems<T>(items: T[], map: T[]): void {
    items.forEach(item => {
        if (!map.includes(item)) {
            map.push(item);
        }
    });
}
