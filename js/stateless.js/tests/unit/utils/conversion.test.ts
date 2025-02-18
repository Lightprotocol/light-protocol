import { describe, it, expect } from 'vitest';
import {
    bufToDecStr,
    hashToBn254FieldSizeBe,
    isSmallerThanBn254FieldSizeBe,
    pushUniqueItems,
    toArray,
    toCamelCase,
} from '../../../src/utils/conversion';
import { calculateComputeUnitPrice } from '../../../src/utils';
import { deserializeAppendNullifyCreateAddressInputsIndexer } from '../../../src/programs';

describe('toArray', () => {
    it('should return same array if array is passed', () => {
        const arr = [1, 2, 3];
        expect(toArray(arr)).toBe(arr);
    });

    it('should wrap non-array in array', () => {
        const value = 42;
        expect(toArray(value)).toEqual([42]);
    });

    describe('calculateComputeUnitPrice', () => {
        it('calculates correct price', () => {
            expect(calculateComputeUnitPrice(1000, 200000)).toBe(5000); // 1000 lamports / 200k CU = 5000 microlamports/CU
            expect(calculateComputeUnitPrice(100, 50000)).toBe(2000); // 100 lamports / 50k CU = 2000 microlamports/CU
            expect(calculateComputeUnitPrice(1, 1000000)).toBe(1); // 1 lamport / 1M CU = 1 microlamport/CU
        });
    });
});

describe('deserialize apc cpi', () => {
    const acp_cpi = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
        0,
    ];

    it('deserialize acp cpi', () => {
        const buffer = Buffer.from(acp_cpi);
        const result =
            deserializeAppendNullifyCreateAddressInputsIndexer(buffer);

        expect(result.meta.is_invoked_by_program).toEqual(1);

        expect(result.addresses.length).toBeGreaterThan(0);
        console.log('address ', result.addresses[0]);
        expect(result.addresses[0]).toEqual({
            address: new Array(32).fill(1),
            tree_index: 1,
            queue_index: 1,
        });

        expect(result.leaves.length).toBeGreaterThan(0);
        expect(result.leaves[0]).toEqual({
            index: 1,
            leaf: new Array(32).fill(1),
        });

        expect(result.nullifiers.length).toBeGreaterThan(0);
        expect(result.nullifiers[0]).toEqual({
            account_hash: new Array(32).fill(1),
            leaf_index: 1,
            prove_by_index: 1,
            tree_index: 1,
            queue_index: 1,
        });
    });
});

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
        152, 39, 87, 169, 208, 143, 101, 43, 128, 245, 59, 22, 134, 182, 231,
        116, 33,
    ];
    const refResult = [
        0, 146, 15, 187, 171, 163, 183, 93, 237, 121, 37, 231, 55, 162, 208,
        188, 244, 77, 185, 157, 93, 9, 101, 193, 220, 247, 109, 94, 48, 212, 98,
        149,
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
        expect(bufToDecStr(Buffer.from([7, 91, 205, 21]))).toEqual('123456789');
    });
});

describe('camelCase', () => {
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

    it('should handle stringified big numbers', () => {
        const input = { big_number: '123456789012345678901234567890' };
        const expected = { bigNumber: '123456789012345678901234567890' };
        expect(toCamelCase(input)).toEqual(expected);
    });

    it('should handle nested objects with stringified big numbers', () => {
        const input = {
            outer_key: { inner_key: '987654321098765432109876543210' },
        };
        const expected = {
            outerKey: { innerKey: '987654321098765432109876543210' },
        };
        expect(toCamelCase(input)).toEqual(expected);
    });

    it('should handle arrays of objects with stringified big numbers', () => {
        const input = [
            { array_key: '12345678901234567890' },
            { another_array_key: '98765432109876543210' },
        ];
        const expected = [
            { arrayKey: '12345678901234567890' },
            { anotherArrayKey: '98765432109876543210' },
        ];
        expect(toCamelCase(input)).toEqual(expected);
    });
});

describe('camelcaseKeys', () => {
    it('should convert snake_case keys to camelCase', () => {
        const originalData = {
            jsonrpc: '2.0',
            result: {
                context: { slot: 5550 },
                value: {
                    items: [
                        {
                            account: {
                                hash: '3WPozgrtYzb25FPo9LYDtvUz22ZY29pU5urzaut6ojc2',
                                address: null,
                                data: {
                                    discriminator: 2,
                                    data: 'BxsGcVK4J4y/Ug1exAUvcRZUveWfMUYTmBn9ECyS1zm/Ux9sAbMniq2HSQoNBVHCfBHcCo0Tr5+d2M9Li+ckd+gDAAAAAAAAAAAA',
                                    data_hash:
                                        'gCvEmNCCr8PYNcdRyiNRbbfnahim17goFhv8cumYc56',
                                },
                                owner: 'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
                                lamports: 0,
                                tree: 'smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT',
                                leafIndex: 122,
                                seq: 123,
                                slot_created: 5550,
                            },
                            token_data: {
                                mint: 'Ujko5vVZpW5SPQRaioe4kEMp6jRQswwVmhJ1vKDXmTv',
                                owner: 'DsrPJH9iwCuDdCoyjnYcscmEUKWtK4CEk3vokVHmpZZY',
                                amount: '1000344548534523434134',
                                delegate: null,
                                state: 'initialized',
                                tlv: null,
                            },
                        },
                    ],
                    cursor: null,
                },
            },
            id: 'test-account',
        };

        const expectedData = {
            jsonrpc: '2.0',
            result: {
                context: { slot: 5550 },
                value: {
                    items: [
                        {
                            account: {
                                hash: '3WPozgrtYzb25FPo9LYDtvUz22ZY29pU5urzaut6ojc2',
                                address: null,
                                data: {
                                    discriminator: 2,
                                    data: 'BxsGcVK4J4y/Ug1exAUvcRZUveWfMUYTmBn9ECyS1zm/Ux9sAbMniq2HSQoNBVHCfBHcCo0Tr5+d2M9Li+ckd+gDAAAAAAAAAAAA',
                                    dataHash:
                                        'gCvEmNCCr8PYNcdRyiNRbbfnahim17goFhv8cumYc56',
                                },
                                owner: 'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
                                lamports: 0,
                                tree: 'smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT',
                                leafIndex: 122,
                                seq: 123,
                                slotCreated: 5550,
                            },
                            tokenData: {
                                mint: 'Ujko5vVZpW5SPQRaioe4kEMp6jRQswwVmhJ1vKDXmTv',
                                owner: 'DsrPJH9iwCuDdCoyjnYcscmEUKWtK4CEk3vokVHmpZZY',
                                amount: '1000344548534523434134',
                                delegate: null,
                                state: 'initialized',
                                tlv: null,
                            },
                        },
                    ],
                    cursor: null,
                },
            },
            id: 'test-account',
        };

        expect(toCamelCase(originalData)).toEqual(expectedData);
    });
});
