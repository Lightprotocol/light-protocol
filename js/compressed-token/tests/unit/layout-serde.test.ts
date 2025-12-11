import { describe, it, expect } from 'vitest';
import { struct, u8, u64 } from '@coral-xyz/borsh';
import { bn } from '@lightprotocol/stateless.js';
import {
    createCompressedAccountDataLayout,
    createDecompressAccountsIdempotentLayout,
    serializeDecompressIdempotentInstructionData,
    deserializeDecompressIdempotentInstructionData,
    CompressedAccountMeta,
    PackedStateTreeInfo,
    CompressedAccountData,
    DecompressAccountsIdempotentInstructionData,
} from '../../src/v3/layout/serde';
import { DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR } from '../../src/constants';

// Simple data layout for testing
const TestDataLayout = struct([u8('value'), u64('amount')]);

// For encoding, Borsh requires BN not native BigInt
interface TestData {
    value: number;
    amount: any; // BN or bigint depending on encode/decode
}

describe('layout-serde', () => {
    describe('createCompressedAccountDataLayout', () => {
        it('should create a layout for compressed account data', () => {
            const layout = createCompressedAccountDataLayout(TestDataLayout);

            expect(layout).toBeDefined();
            expect(typeof layout.encode).toBe('function');
            expect(typeof layout.decode).toBe('function');
        });

        it('should encode and decode compressed account data with empty seeds', () => {
            const layout = createCompressedAccountDataLayout(TestDataLayout);

            const meta: CompressedAccountMeta = {
                treeInfo: {
                    rootIndex: 10,
                    proveByIndex: true,
                    merkleTreePubkeyIndex: 1,
                    queuePubkeyIndex: 2,
                    leafIndex: 100,
                },
                address: Array(32).fill(42),
                lamports: bn(1000000),
                outputStateTreeIndex: 3,
            };

            const testData: TestData = {
                value: 255,
                amount: bn(500),
            };

            const accountData: CompressedAccountData<TestData> = {
                meta,
                data: testData,
                seeds: [], // Empty seeds to avoid Buffer conversion issues
            };

            const buffer = Buffer.alloc(500);
            const len = layout.encode(accountData, buffer);
            const decoded = layout.decode(buffer.subarray(0, len));

            expect(decoded.meta.treeInfo.rootIndex).toBe(10);
            expect(decoded.meta.treeInfo.proveByIndex).toBe(true);
            expect(decoded.meta.treeInfo.leafIndex).toBe(100);
            expect(decoded.meta.address).toEqual(Array(32).fill(42));
            expect(decoded.meta.outputStateTreeIndex).toBe(3);
            expect(decoded.data.value).toBe(255);
            expect(decoded.seeds.length).toBe(0);
        });

        it('should handle null address and lamports', () => {
            const layout = createCompressedAccountDataLayout(TestDataLayout);

            const meta: CompressedAccountMeta = {
                treeInfo: {
                    rootIndex: 5,
                    proveByIndex: false,
                    merkleTreePubkeyIndex: 0,
                    queuePubkeyIndex: 1,
                    leafIndex: 50,
                },
                address: null,
                lamports: null,
                outputStateTreeIndex: 0,
            };

            const accountData: CompressedAccountData<TestData> = {
                meta,
                data: { value: 128, amount: bn(200) },
                seeds: [],
            };

            const buffer = Buffer.alloc(500);
            const len = layout.encode(accountData, buffer);
            const decoded = layout.decode(buffer.subarray(0, len));

            expect(decoded.meta.address).toBe(null);
            expect(decoded.meta.lamports).toBe(null);
        });
    });

    describe('createDecompressAccountsIdempotentLayout', () => {
        it('should create a layout for decompress instruction', () => {
            const layout =
                createDecompressAccountsIdempotentLayout(TestDataLayout);

            expect(layout).toBeDefined();
            expect(typeof layout.encode).toBe('function');
            expect(typeof layout.decode).toBe('function');
        });
    });

    describe('serializeDecompressIdempotentInstructionData / deserializeDecompressIdempotentInstructionData', () => {
        it('should serialize and deserialize instruction data with empty seeds', () => {
            const proof = {
                a: Array(32).fill(1),
                b: Array(64).fill(2),
                c: Array(32).fill(3),
            };

            const meta: CompressedAccountMeta = {
                treeInfo: {
                    rootIndex: 15,
                    proveByIndex: true,
                    merkleTreePubkeyIndex: 2,
                    queuePubkeyIndex: 3,
                    leafIndex: 200,
                },
                address: Array(32).fill(99),
                lamports: bn(5000000),
                outputStateTreeIndex: 1,
            };

            const testData: TestData = {
                value: 42,
                amount: bn(1000),
            };

            const compressedAccount: CompressedAccountData<TestData> = {
                meta,
                data: testData,
                seeds: [], // Empty seeds to avoid Buffer issues
            };

            const data: DecompressAccountsIdempotentInstructionData<TestData> =
                {
                    proof,
                    compressedAccounts: [compressedAccount],
                    systemAccountsOffset: 5,
                };

            const serialized = serializeDecompressIdempotentInstructionData(
                data,
                TestDataLayout,
            );

            // Check discriminator
            expect(
                serialized.subarray(
                    0,
                    DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR.length,
                ),
            ).toEqual(DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR);

            const deserialized =
                deserializeDecompressIdempotentInstructionData<TestData>(
                    serialized,
                    TestDataLayout,
                );

            expect(deserialized.proof.a).toEqual(Array(32).fill(1));
            expect(deserialized.proof.b).toEqual(Array(64).fill(2));
            expect(deserialized.proof.c).toEqual(Array(32).fill(3));
            expect(deserialized.compressedAccounts.length).toBe(1);
            expect(deserialized.systemAccountsOffset).toBe(5);

            const decompressedAccount = deserialized.compressedAccounts[0];
            expect(decompressedAccount.meta.treeInfo.rootIndex).toBe(15);
            expect(decompressedAccount.meta.treeInfo.leafIndex).toBe(200);
            expect(decompressedAccount.data.value).toBe(42);
        });

        it('should handle multiple compressed accounts', () => {
            const proof = {
                a: Array(32).fill(0),
                b: Array(64).fill(0),
                c: Array(32).fill(0),
            };

            const accounts: CompressedAccountData<TestData>[] = [
                {
                    meta: {
                        treeInfo: {
                            rootIndex: 1,
                            proveByIndex: true,
                            merkleTreePubkeyIndex: 0,
                            queuePubkeyIndex: 1,
                            leafIndex: 10,
                        },
                        address: null,
                        lamports: null,
                        outputStateTreeIndex: 0,
                    },
                    data: { value: 1, amount: bn(100) },
                    seeds: [],
                },
                {
                    meta: {
                        treeInfo: {
                            rootIndex: 2,
                            proveByIndex: false,
                            merkleTreePubkeyIndex: 1,
                            queuePubkeyIndex: 2,
                            leafIndex: 20,
                        },
                        address: Array(32).fill(1),
                        lamports: bn(1000),
                        outputStateTreeIndex: 1,
                    },
                    data: { value: 2, amount: bn(200) },
                    seeds: [],
                },
                {
                    meta: {
                        treeInfo: {
                            rootIndex: 3,
                            proveByIndex: true,
                            merkleTreePubkeyIndex: 2,
                            queuePubkeyIndex: 3,
                            leafIndex: 30,
                        },
                        address: Array(32).fill(2),
                        lamports: bn(2000),
                        outputStateTreeIndex: 2,
                    },
                    data: { value: 3, amount: bn(300) },
                    seeds: [],
                },
            ];

            const data: DecompressAccountsIdempotentInstructionData<TestData> =
                {
                    proof,
                    compressedAccounts: accounts,
                    systemAccountsOffset: 10,
                };

            const serialized = serializeDecompressIdempotentInstructionData(
                data,
                TestDataLayout,
            );

            const deserialized =
                deserializeDecompressIdempotentInstructionData<TestData>(
                    serialized,
                    TestDataLayout,
                );

            expect(deserialized.compressedAccounts.length).toBe(3);
            expect(deserialized.compressedAccounts[0].data.value).toBe(1);
            expect(deserialized.compressedAccounts[1].data.value).toBe(2);
            expect(deserialized.compressedAccounts[2].data.value).toBe(3);
            expect(
                deserialized.compressedAccounts[0].meta.treeInfo.leafIndex,
            ).toBe(10);
            expect(
                deserialized.compressedAccounts[1].meta.treeInfo.leafIndex,
            ).toBe(20);
            expect(
                deserialized.compressedAccounts[2].meta.treeInfo.leafIndex,
            ).toBe(30);
        });

        it('should handle empty compressed accounts array', () => {
            const proof = {
                a: Array(32).fill(5),
                b: Array(64).fill(6),
                c: Array(32).fill(7),
            };

            const data: DecompressAccountsIdempotentInstructionData<TestData> =
                {
                    proof,
                    compressedAccounts: [],
                    systemAccountsOffset: 0,
                };

            const serialized = serializeDecompressIdempotentInstructionData(
                data,
                TestDataLayout,
            );

            const deserialized =
                deserializeDecompressIdempotentInstructionData<TestData>(
                    serialized,
                    TestDataLayout,
                );

            expect(deserialized.compressedAccounts.length).toBe(0);
            expect(deserialized.systemAccountsOffset).toBe(0);
        });

        it('should handle various systemAccountsOffset values', () => {
            const proof = {
                a: Array(32).fill(0),
                b: Array(64).fill(0),
                c: Array(32).fill(0),
            };

            // Test boundary values
            const offsets = [0, 1, 127, 255];

            for (const offset of offsets) {
                const data: DecompressAccountsIdempotentInstructionData<TestData> =
                    {
                        proof,
                        compressedAccounts: [],
                        systemAccountsOffset: offset,
                    };

                const serialized = serializeDecompressIdempotentInstructionData(
                    data,
                    TestDataLayout,
                );

                const deserialized =
                    deserializeDecompressIdempotentInstructionData<TestData>(
                        serialized,
                        TestDataLayout,
                    );

                expect(deserialized.systemAccountsOffset).toBe(offset);
            }
        });
    });

    describe('PackedStateTreeInfo', () => {
        it('should correctly handle all tree info fields', () => {
            const layout = createCompressedAccountDataLayout(TestDataLayout);

            const treeInfo: PackedStateTreeInfo = {
                rootIndex: 65535, // max u16
                proveByIndex: true,
                merkleTreePubkeyIndex: 255, // max u8
                queuePubkeyIndex: 255, // max u8
                leafIndex: 4294967295, // max u32
            };

            const meta: CompressedAccountMeta = {
                treeInfo,
                address: null,
                lamports: null,
                outputStateTreeIndex: 255,
            };

            const accountData: CompressedAccountData<TestData> = {
                meta,
                data: { value: 0, amount: bn(0) },
                seeds: [],
            };

            const buffer = Buffer.alloc(500);
            const len = layout.encode(accountData, buffer);
            const decoded = layout.decode(buffer.subarray(0, len));

            expect(decoded.meta.treeInfo.rootIndex).toBe(65535);
            expect(decoded.meta.treeInfo.proveByIndex).toBe(true);
            expect(decoded.meta.treeInfo.merkleTreePubkeyIndex).toBe(255);
            expect(decoded.meta.treeInfo.queuePubkeyIndex).toBe(255);
            expect(decoded.meta.treeInfo.leafIndex).toBe(4294967295);
            expect(decoded.meta.outputStateTreeIndex).toBe(255);
        });
    });
});
