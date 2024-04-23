import { Buffer } from 'buffer';
import { bn, createBN254 } from '../state/BN254';
import { FIELD_SIZE } from '../constants';
import { keccak_256 } from '@noble/hashes/sha3';
import { Keypair } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';

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
function isSmallerThanBn254FieldSizeBe(bytes: Buffer): boolean {
    const bigint = bn(bytes, undefined, 'be');
    return bigint.lt(FIELD_SIZE);
}

export async function hashToBn254FieldSizeBe(
    bytes: Buffer,
): Promise<[Buffer, number] | null> {
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

/** Mutates array in place */
export function pushUniqueItems<T>(items: T[], map: T[]): void {
    items.forEach(item => {
        if (!map.includes(item)) {
            map.push(item);
        }
    });
}

export function toCamelCase(
    obj: Array<any> | unknown | any,
): Array<any> | unknown | any {
    if (Array.isArray(obj)) {
        return obj.map(v => toCamelCase(v));
    } else if (obj !== null && obj.constructor === Object) {
        return Object.keys(obj).reduce((result, key) => {
            const camelCaseKey = key.replace(/([-_][a-z])/gi, $1 => {
                return $1.toUpperCase().replace('-', '').replace('_', '');
            });
            result[camelCaseKey] = toCamelCase(obj[key]);
            return result;
        }, {} as any);
    }
    return obj;
}

// FIXME: check bundling and how to resolve the type error
//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    describe('toArray function', () => {
        it('should convert a single item to an array', () => {
            expect(toArray(1)).toEqual([1]);
        });

        it('should leave an array unchanged', () => {
            expect(toArray([1, 2, 3])).toEqual([1, 2, 3]);
        });
    });

    describe('isSmallerThanBn254FieldSizeBe function', () => {
        it('should return true for a small number', () => {
            const buf = Buffer.from(
                '0000000000000000000000000000000000000000000000000000000000000000',
                'hex',
            );
            expect(isSmallerThanBn254FieldSizeBe(buf)).toBe(true);
        });

        it('should return false for a large number', () => {
            const buf = Buffer.from(
                '0000000000000000000000000000000000000000000000000000000000000065',
                'hex',
            ).reverse();
            expect(isSmallerThanBn254FieldSizeBe(buf)).toBe(false);
        });
    });

    describe('hashToBn254FieldSizeBe function', () => {
        const refBumpSeed = [252];
        const bytes = [
            131, 219, 249, 246, 221, 196, 33, 3, 114, 23, 121, 235, 18, 229, 71,
            152, 39, 87, 169, 208, 143, 101, 43, 128, 245, 59, 22, 134, 182,
            231, 116, 33,
        ];
        const refResult = [
            0, 146, 15, 187, 171, 163, 183, 93, 237, 121, 37, 231, 55, 162, 208,
            188, 244, 77, 185, 157, 93, 9, 101, 193, 220, 247, 109, 94, 48, 212,
            98, 149,
        ];

        it('should return a valid value for initial buffer', async () => {
            const result = await hashToBn254FieldSizeBe(Buffer.from(bytes));
            expect(Array.from(result![0])).toEqual(refResult);
        });

        it('should return a valid value for initial buffer', async () => {
            const buf = Buffer.from(
                '0000000000000000000000000000000000000000000000000000000000000000',
                'hex',
            );
            const result = await hashToBn254FieldSizeBe(buf);
            expect(result).not.toBeNull();
            if (result) {
                expect(result[0]).toBeInstanceOf(Buffer);
                expect(result[1]).toBe(255);
            }
        });

        it('should return a valid value for a buffer that can be hashed to a smaller value', async () => {
            const buf = Buffer.from(
                'fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe',
                'hex',
            );
            const result = await hashToBn254FieldSizeBe(buf);
            expect(result).not.toBeNull();
            if (result) {
                expect(result[1]).toBeLessThanOrEqual(255);
                expect(result[0]).toBeInstanceOf(Buffer);
                // Check if the hashed value is indeed smaller than the bn254 field size
                expect(isSmallerThanBn254FieldSizeBe(result[0])).toBe(true);
            }
        });

        it('should correctly hash the input buffer', async () => {
            const buf = Buffer.from('deadbeef', 'hex');
            const result = await hashToBn254FieldSizeBe(buf);
            expect(result).not.toBeNull();
            if (result) {
                // Since the actual hash value depends on the crypto implementation and input,
                // we cannot predict the exact output. However, we can check if the output is valid.
                expect(result[0].length).toBe(32); // SHA-256 hash length
                expect(result[1]).toBeLessThanOrEqual(255);
                expect(isSmallerThanBn254FieldSizeBe(result[0])).toBe(true);
            }
        });
    });

    describe('pushUniqueItems function', () => {
        it('should add unique items', () => {
            const map = [1, 2, 3];
            const itemsToAdd = [3, 4, 5];
            pushUniqueItems(itemsToAdd, map);
            expect(map).toEqual([1, 2, 3, 4, 5]);
        });

        it('should ignore duplicates', () => {
            const map = [1, 2, 3];
            const itemsToAdd = [1, 2, 3];
            pushUniqueItems(itemsToAdd, map);
            expect(map).toEqual([1, 2, 3]);
        });

        it('should handle empty arrays', () => {
            const map: number[] = [];
            const itemsToAdd: number[] = [];
            pushUniqueItems(itemsToAdd, map);
            expect(map).toEqual([]);
        });
    });

    describe('bufToDecStr', () => {
        it("should convert buffer [0] to '0'", () => {
            expect(bufToDecStr(Buffer.from([0]))).toEqual('0');
        });

        it("should convert buffer [1] to '1'", () => {
            expect(bufToDecStr(Buffer.from([1]))).toEqual('1');
        });

        it("should convert buffer [1, 0] to '256'", () => {
            expect(bufToDecStr(Buffer.from([1, 0]))).toEqual('256');
        });

        it("should convert buffer [1, 1] to '257'", () => {
            expect(bufToDecStr(Buffer.from([1, 1]))).toEqual('257');
        });

        it("should convert buffer [7, 91, 205, 21] to '123456789'", () => {
            expect(bufToDecStr(Buffer.from([7, 91, 205, 21]))).toEqual(
                '123456789',
            );
        });
    });

    describe('toCamelCase', () => {
        it('should convert object keys to camelCase', () => {
            const input = { test_key: 1, 'another-testKey': 2 };
            const expected = { testKey: 1, anotherTestKey: 2 };
            expect(toCamelCase(input)).toEqual(expected);
        });

        it('should handle arrays of objects', () => {
            const input = [{ array_key: 3 }, { 'another_array-key': 4 }];
            const expected = [{ arrayKey: 3 }, { anotherArrayKey: 4 }];
            expect(toCamelCase(input)).toEqual(expected);
        });

        it('should return the input if it is neither an object nor an array', () => {
            const input = 'testString';
            expect(toCamelCase(input)).toBe(input);
        });
    });
}
