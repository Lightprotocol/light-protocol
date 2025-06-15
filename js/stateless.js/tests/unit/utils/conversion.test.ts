import { describe, it, expect } from 'vitest';
import {
    bufToDecStr,
    hashToBn254FieldSizeBe,
    isSmallerThanBn254FieldSizeBe,
    pushUniqueItems,
    toArray,
    toCamelCase,
    convertInvokeCpiWithReadOnlyToInvoke,
} from '../../../src/utils/conversion';
import { calculateComputeUnitPrice } from '../../../src/utils';
import { deserializeAppendNullifyCreateAddressInputsIndexer } from '../../../src/programs';
import { decodeInstructionDataInvokeCpiWithReadOnly } from '../../../src/programs/system/layout';
import BN from 'bn.js';

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
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    it('deserialize acp cpi', () => {
        const buffer = Buffer.from(acp_cpi);
        const result =
            deserializeAppendNullifyCreateAddressInputsIndexer(buffer);

        expect(result.meta.is_invoked_by_program).toEqual(1);

        expect(result.addresses.length).toBeGreaterThan(0);

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

describe('deserialize InstructionDataInvokeCpiWithReadOnly', () => {
    it('should deserialize the complete InstructionDataInvokeCpiWithReadOnly structure', () => {
        // first 8 bytes are skipped.
        const data = [
            1, 0, 0, 0, 1, 0, 0, 0, 0, 148, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90, 70,
            83, 164, 216, 39, 10, 106, 0, 0, 1, 0, 1, 83, 0, 3, 0, 0, 0, 91, 97,
            69, 180, 246, 54, 236, 250, 62, 116, 95, 226, 176, 250, 172, 150,
            38, 157, 38, 110, 3, 110, 130, 133, 102, 14, 42, 118, 151, 177, 74,
            49, 180, 127, 245, 54, 1, 13, 208, 197, 129, 101, 36, 193, 85, 161,
            48, 175, 182, 23, 26, 150, 52, 204, 60, 96, 233, 248, 140, 33, 212,
            16, 175, 111, 218, 54, 195, 97, 239, 148, 66, 48, 24, 183, 0, 254,
            113, 31, 157, 136, 188, 202, 183, 37, 203, 248, 36, 216, 177, 227,
            159, 93, 238, 171, 167, 173, 224, 196, 144, 193, 203, 88, 88, 133,
            174, 71, 142, 254, 17, 121, 254, 208, 0, 153, 1, 0, 0, 0, 237, 83,
            2, 61, 227, 140, 40, 48, 68, 54, 55, 57, 228, 108, 104, 1, 19, 138,
            156, 96, 249, 111, 250, 212, 130, 57, 47, 54, 4, 5, 48, 192, 174,
            157, 141, 112, 18, 255, 0, 64, 136, 164, 130, 37, 210, 47, 0, 253,
            75, 4, 203, 167, 187, 45, 253, 192, 154, 0, 4, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 201, 78, 254, 108, 214, 2, 223, 68, 0, 0, 43, 0, 0,
            0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 78, 17, 123, 28, 100, 171, 124, 219, 0, 0, 253,
            0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 126, 220, 103, 34, 32, 110, 222, 30, 0,
            0, 197, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 196, 198, 75, 26, 237, 186, 126,
            74, 0, 1, 19, 61, 250, 254, 150, 6, 163, 86, 0, 0, 0, 0, 156, 9, 53,
            70, 77, 194, 172, 226, 190, 160, 23, 141, 31, 196, 236, 120, 84,
            107, 116, 110, 205, 212, 164, 48, 143, 224, 119, 115, 144, 225, 207,
            228, 49, 3, 0, 0, 0, 39, 168, 127, 189, 18, 209, 50, 130, 61, 249,
            224, 77, 91, 119, 75, 140, 171, 218, 60, 106, 84, 193, 224, 111,
            159, 45, 25, 182, 255, 151, 70, 104, 70, 51, 175, 83, 83, 120, 178,
            62, 215, 154, 181, 237, 76, 231, 56, 133, 102, 223, 246, 189, 104,
            18, 195, 42, 151, 220, 240, 78, 245, 64, 112, 90, 139, 200, 70, 9,
            144, 245, 142, 205, 162, 130, 217, 110, 191, 231, 184, 36, 71, 173,
            105, 78, 104, 199, 27, 1, 160, 6, 177, 68, 34, 22, 224, 174, 159,
            50, 42, 53, 143, 251, 61, 65, 82, 2, 0, 0, 0, 139, 161, 56, 237,
            157, 233, 116, 185, 12, 196, 217, 30, 184, 96, 146, 164, 150, 251,
            140, 3, 158, 71, 77, 130, 169, 233, 128, 60, 221, 108, 98, 247, 124,
            28, 145, 30, 204, 146, 1, 14, 104, 21, 236, 252, 114, 187, 150, 4,
            37, 93, 254, 107, 46, 123, 96, 206, 209, 39, 91, 61, 214, 71, 4,
            118, 24, 221, 216, 152, 135, 71, 93, 155, 81, 50, 14, 128, 30, 108,
            170, 1, 235, 59,
        ];

        const buffer = Buffer.from(data);
        const result = decodeInstructionDataInvokeCpiWithReadOnly(buffer);

        // Build the expected struct that accurately maps to the Rust structure
        const expectedStruct = {
            mode: 0,
            bump: 148,
            invoking_program_id: expect.any(Object), // PublicKey object
            compress_or_decompress_lamports: expect.any(Object), // BN object equal to 7640963529210807898
            is_compress: false,
            with_cpi_context: false,
            with_transaction_hash: true,
            compressedCpiContext: {
                set_context: false,
                first_set_context: true,
                cpi_context_account_index: 83,
            },
            proof: null,
            new_address_params: [
                {
                    seed: [
                        91, 97, 69, 180, 246, 54, 236, 250, 62, 116, 95, 226,
                        176, 250, 172, 150, 38, 157, 38, 110, 3, 110, 130, 133,
                        102, 14, 42, 118, 151, 177, 74, 49,
                    ],
                    address_queue_account_index: 180,
                    address_merkle_tree_account_index: 127,
                    address_merkle_tree_root_index: 14069,
                    assigned_to_account: true,
                    assigned_account_index: 13,
                },
                {
                    seed: [
                        208, 197, 129, 101, 36, 193, 85, 161, 48, 175, 182, 23,
                        26, 150, 52, 204, 60, 96, 233, 248, 140, 33, 212, 16,
                        175, 111, 218, 54, 195, 97, 239, 148,
                    ],
                    address_queue_account_index: 66,
                    address_merkle_tree_account_index: 48,
                    address_merkle_tree_root_index: 46872,
                    assigned_to_account: false,
                    assigned_account_index: 254,
                },
                {
                    seed: [
                        113, 31, 157, 136, 188, 202, 183, 37, 203, 248, 36, 216,
                        177, 227, 159, 93, 238, 171, 167, 173, 224, 196, 144,
                        193, 203, 88, 88, 133, 174, 71, 142, 254,
                    ],
                    address_queue_account_index: 17,
                    address_merkle_tree_account_index: 121,
                    address_merkle_tree_root_index: 53502,
                    assigned_to_account: false,
                    assigned_account_index: 153,
                },
            ],
            input_compressed_accounts: [
                {
                    discriminator: [237, 83, 2, 61, 227, 140, 40, 48],
                    data_hash: [
                        68, 54, 55, 57, 228, 108, 104, 1, 19, 138, 156, 96, 249,
                        111, 250, 212, 130, 57, 47, 54, 4, 5, 48, 192, 174, 157,
                        141, 112, 18, 255, 0, 64,
                    ],
                    packedMerkleContext: {
                        merkle_tree_pubkey_index: 136,
                        queue_pubkey_index: 164,
                        leaf_index: 802301314,
                        prove_by_index: false,
                    },
                    root_index: 19453,
                    lamports: expect.any(Object), // BN for 11151191050233039620
                    address: null,
                },
            ],
            output_compressed_accounts: [
                {
                    compressedAccount: {
                        owner: expect.any(Object), // PublicKey
                        lamports: expect.any(Object), // BN
                        address: null,
                        data: null,
                    },
                    merkleTreeIndex: 43,
                },
                {
                    compressedAccount: {
                        owner: expect.any(Object), // PublicKey
                        lamports: expect.any(Object), // BN
                        address: null,
                        data: null,
                    },
                    merkleTreeIndex: 253,
                },
                {
                    compressedAccount: {
                        owner: expect.any(Object), // PublicKey
                        lamports: expect.any(Object), // BN
                        address: null,
                        data: null,
                    },
                    merkleTreeIndex: 197,
                },
                {
                    compressedAccount: {
                        owner: expect.any(Object), // PublicKey
                        lamports: expect.any(Object), // BN
                        address: null,
                        data: expect.any(Object), // This one has data
                    },
                    merkleTreeIndex: 49,
                },
            ],
            read_only_addresses: [
                {
                    address: [
                        39, 168, 127, 189, 18, 209, 50, 130, 61, 249, 224, 77,
                        91, 119, 75, 140, 171, 218, 60, 106, 84, 193, 224, 111,
                        159, 45, 25, 182, 255, 151, 70, 104,
                    ],
                    address_merkle_tree_root_index: 13126,
                    address_merkle_tree_account_index: 175,
                },
                {
                    address: [
                        83, 83, 120, 178, 62, 215, 154, 181, 237, 76, 231, 56,
                        133, 102, 223, 246, 189, 104, 18, 195, 42, 151, 220,
                        240, 78, 245, 64, 112, 90, 139, 200, 70,
                    ],
                    address_merkle_tree_root_index: 36873,
                    address_merkle_tree_account_index: 245,
                },
                {
                    address: [
                        142, 205, 162, 130, 217, 110, 191, 231, 184, 36, 71,
                        173, 105, 78, 104, 199, 27, 1, 160, 6, 177, 68, 34, 22,
                        224, 174, 159, 50, 42, 53, 143, 251,
                    ],
                    address_merkle_tree_root_index: 16701,
                    address_merkle_tree_account_index: 82,
                },
            ],
            read_only_accounts: [
                {
                    account_hash: [
                        139, 161, 56, 237, 157, 233, 116, 185, 12, 196, 217, 30,
                        184, 96, 146, 164, 150, 251, 140, 3, 158, 71, 77, 130,
                        169, 233, 128, 60, 221, 108, 98, 247,
                    ],
                    packedMerkleContext: {
                        merkle_tree_pubkey_index: 124,
                        queue_pubkey_index: 28,
                        leaf_index: 2462850705,
                        prove_by_index: true,
                    },
                    root_index: 26638,
                },
                {
                    account_hash: [
                        21, 236, 252, 114, 187, 150, 4, 37, 93, 254, 107, 46,
                        123, 96, 206, 209, 39, 91, 61, 214, 71, 4, 118, 24, 221,
                        216, 152, 135, 71, 93, 155, 81,
                    ],
                    packedMerkleContext: {
                        merkle_tree_pubkey_index: 50,
                        queue_pubkey_index: 14,
                        leaf_index: 2859212416,
                        prove_by_index: true,
                    },
                    root_index: 15339,
                },
            ],
        };

        // Assert all fields in our expected struct match the result
        // Use partial matching since the input buffer may not contain all fields
        expect(result).toMatchObject(expectedStruct);
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

describe('convertInvokeCpiWithReadOnlyToInvoke', () => {
    it('should convert InstructionDataInvokeCpiWithReadOnly to InstructionDataInvoke', () => {
        const mockCpiWithReadOnly = {
            mode: 0,
            bump: 148,
            invoking_program_id: { toBuffer: () => Buffer.alloc(32) },
            compress_or_decompress_lamports: new BN(1000),
            is_compress: true,
            with_cpi_context: false,
            with_transaction_hash: true,
            compressedCpiContext: {
                set_context: false,
                first_set_context: true,
                cpi_context_account_index: 83,
            },
            proof: null,
            new_address_params: [
                {
                    seed: Array(32).fill(1),
                    address_queue_account_index: 5,
                    address_merkle_tree_account_index: 6,
                    address_merkle_tree_root_index: 123,
                    assigned_to_account: true,
                    assigned_account_index: 7,
                },
            ],
            input_compressed_accounts: [
                {
                    discriminator: Array(8).fill(2),
                    data_hash: Array(32).fill(3),
                    packedMerkleContext: {
                        merkle_tree_pubkey_index: 8,
                        queue_pubkey_index: 9,
                        leaf_index: 456,
                        prove_by_index: false,
                    },
                    root_index: 789,
                    lamports: new BN(2000),
                    address: null,
                },
            ],
            output_compressed_accounts: [
                {
                    compressedAccount: {
                        owner: { toBuffer: () => Buffer.alloc(32) },
                        lamports: new BN(3000),
                        address: null,
                        data: null,
                    },
                    merkleTreeIndex: 10,
                },
            ],
            read_only_addresses: [
                {
                    address: Array(32).fill(4),
                    address_merkle_tree_root_index: 567,
                    address_merkle_tree_account_index: 11,
                },
            ],
            read_only_accounts: [
                {
                    account_hash: Array(32).fill(5),
                    packedMerkleContext: {
                        merkle_tree_pubkey_index: 12,
                        queue_pubkey_index: 13,
                        leaf_index: 890,
                        prove_by_index: true,
                    },
                    root_index: 321,
                },
            ],
        };

        // Convert to InstructionDataInvoke
        const result =
            convertInvokeCpiWithReadOnlyToInvoke(mockCpiWithReadOnly);

        // Verify the result is an InstructionDataInvoke
        expect(result).toBeDefined();
        expect(result.proof).toBeNull();
        expect(result.isCompress).toBe(true);
        expect(result.compressOrDecompressLamports).toEqual(new BN(1000));

        // Check newAddressParams conversion
        expect(result.newAddressParams).toHaveLength(1);
        expect(result.newAddressParams[0].seed).toEqual(Array(32).fill(1));
        expect(result.newAddressParams[0].addressQueueAccountIndex).toBe(5);
        expect(result.newAddressParams[0].addressMerkleTreeAccountIndex).toBe(
            6,
        );
        expect(result.newAddressParams[0].addressMerkleTreeRootIndex).toBe(123);

        // Check input accounts conversion
        expect(result.inputCompressedAccountsWithMerkleContext).toHaveLength(1);

        // First account (from input_compressed_accounts)
        const firstAccount = result.inputCompressedAccountsWithMerkleContext[0];
        expect(firstAccount.rootIndex).toBe(789);

        expect(firstAccount.readOnly).toBe(false);
        expect(firstAccount.compressedAccount.lamports).toEqual(new BN(2000));
        expect(firstAccount.merkleContext.merkleTreePubkeyIndex).toBe(8);
        expect(firstAccount.merkleContext.queuePubkeyIndex).toBe(9);
        expect(firstAccount.merkleContext.leafIndex).toBe(456);
        expect(firstAccount.merkleContext.proveByIndex).toBe(false);
        // Check output accounts conversion
        expect(result.outputCompressedAccounts).toHaveLength(1);
        expect(result.outputCompressedAccounts[0].merkleTreeIndex).toBe(10);
        expect(
            result.outputCompressedAccounts[0].compressedAccount.lamports,
        ).toEqual(new BN(3000));
    });
});
