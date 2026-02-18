import { describe, it, expect } from 'vitest';
import {
    encodeTransfer2InstructionData,
    createCompressSpl,
    createDecompressCtoken,
    createDecompressSpl,
    Transfer2InstructionData,
    Compression,
    TRANSFER2_DISCRIMINATOR,
    COMPRESSION_MODE_COMPRESS,
    COMPRESSION_MODE_DECOMPRESS,
} from '../../src/v3/layout/layout-transfer2';
import { MAX_TOP_UP } from '../../src/constants';

describe('layout-transfer2', () => {
    describe('encodeTransfer2InstructionData', () => {
        it('should encode basic transfer instruction data', () => {
            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions: null,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            // Check discriminator
            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
            expect(encoded.length).toBeGreaterThan(1);
        });

        it('should encode maxTopUp MAX_TOP_UP (65535) and round-trip at offset 6', () => {
            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: MAX_TOP_UP,
                cpiContext: null,
                compressions: null,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
            expect(encoded.readUInt16LE(6)).toBe(65535);
        });

        it('should encode with compressions array', () => {
            const compressions: Compression[] = [
                {
                    mode: COMPRESSION_MODE_COMPRESS,
                    amount: 1000n,
                    mint: 0,
                    sourceOrRecipient: 1,
                    authority: 2,
                    poolAccountIndex: 3,
                    poolIndex: 0,
                    bump: 255,
                    decimals: 9,
                },
            ];

            const data: Transfer2InstructionData = {
                withTransactionHash: true,
                withLamportsChangeAccountMerkleTreeIndex: true,
                lamportsChangeAccountMerkleTreeIndex: 5,
                lamportsChangeAccountOwnerIndex: 3,
                outputQueue: 1,
                maxTopUp: 100,
                cpiContext: null,
                compressions,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
            // Encoding with compressions should produce larger buffer
            expect(encoded.length).toBeGreaterThan(20);
        });

        it('should encode with input and output token data', () => {
            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions: null,
                proof: null,
                inTokenData: [
                    {
                        owner: 0,
                        amount: 500n,
                        hasDelegate: false,
                        delegate: 0,
                        mint: 1,
                        version: 1,
                        merkleContext: {
                            merkleTreePubkeyIndex: 2,
                            queuePubkeyIndex: 3,
                            leafIndex: 100,
                            proveByIndex: true,
                        },
                        rootIndex: 5,
                    },
                ],
                outTokenData: [
                    {
                        owner: 1,
                        amount: 500n,
                        hasDelegate: false,
                        delegate: 0,
                        mint: 1,
                        version: 1,
                    },
                ],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
        });

        it('should encode with proof', () => {
            const proof = {
                a: Array(32).fill(1),
                b: Array(64).fill(2),
                c: Array(32).fill(3),
            };

            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions: null,
                proof,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
            // Proof adds 128 bytes
            expect(encoded.length).toBeGreaterThan(128);
        });

        it('should encode with cpiContext', () => {
            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: {
                    setContext: true,
                    firstSetContext: true,
                    cpiContextAccountIndex: 5,
                },
                compressions: null,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
        });

        it('should encode with lamports arrays', () => {
            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions: null,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: [1000000n, 2000000n],
                outLamports: [3000000n],
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
        });

        it('should handle large amount values', () => {
            const largeAmount = BigInt('18446744073709551615'); // max u64

            const compressions: Compression[] = [
                {
                    mode: COMPRESSION_MODE_COMPRESS,
                    amount: largeAmount,
                    mint: 0,
                    sourceOrRecipient: 1,
                    authority: 2,
                    poolAccountIndex: 3,
                    poolIndex: 0,
                    bump: 255,
                    decimals: 9,
                },
            ];

            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            // Should not throw
            const encoded = encodeTransfer2InstructionData(data);
            expect(encoded.length).toBeGreaterThan(1);
        });
    });

    describe('createCompressSpl', () => {
        it('should create compression struct for SPL wrap', () => {
            const compression = createCompressSpl(
                1000n, // amount
                0, // mintIndex
                1, // sourceIndex
                2, // authorityIndex
                3, // poolAccountIndex
                0, // poolIndex
                255, // bump
                9, // decimals
            );

            expect(compression.mode).toBe(COMPRESSION_MODE_COMPRESS);
            expect(compression.amount).toBe(1000n);
            expect(compression.mint).toBe(0);
            expect(compression.sourceOrRecipient).toBe(1);
            expect(compression.authority).toBe(2);
            expect(compression.poolAccountIndex).toBe(3);
            expect(compression.poolIndex).toBe(0);
            expect(compression.bump).toBe(255);
            expect(compression.decimals).toBe(9);
        });

        it('should handle different index values', () => {
            const compression = createCompressSpl(
                500n,
                5, // mintIndex
                10, // sourceIndex
                15, // authorityIndex
                20, // poolAccountIndex
                3, // poolIndex
                254, // bump
                6, // decimals
            );

            expect(compression.mint).toBe(5);
            expect(compression.sourceOrRecipient).toBe(10);
            expect(compression.authority).toBe(15);
            expect(compression.poolAccountIndex).toBe(20);
            expect(compression.poolIndex).toBe(3);
            expect(compression.bump).toBe(254);
        });

        it('should handle large amounts', () => {
            const largeAmount = BigInt('18446744073709551615');
            const compression = createCompressSpl(
                largeAmount,
                0,
                1,
                2,
                3,
                0,
                255,
                9,
            );

            expect(compression.amount).toBe(largeAmount);
        });
    });

    describe('createDecompressCtoken', () => {
        it('should create decompression struct for CToken', () => {
            const decompression = createDecompressCtoken(
                2000n, // amount
                0, // mintIndex
                1, // recipientIndex
                5, // tokenProgramIndex
            );

            expect(decompression.mode).toBe(COMPRESSION_MODE_DECOMPRESS);
            expect(decompression.amount).toBe(2000n);
            expect(decompression.mint).toBe(0);
            expect(decompression.sourceOrRecipient).toBe(1);
            expect(decompression.authority).toBe(0);
            expect(decompression.poolAccountIndex).toBe(5);
            expect(decompression.poolIndex).toBe(0);
            expect(decompression.bump).toBe(0);
            expect(decompression.decimals).toBe(0);
        });

        it('should use default tokenProgramIndex when not provided', () => {
            const decompression = createDecompressCtoken(
                1000n, // amount
                0, // mintIndex
                1, // recipientIndex
                // tokenProgramIndex not provided
            );

            expect(decompression.poolAccountIndex).toBe(0);
        });

        it('should handle different amounts', () => {
            const decompression = createDecompressCtoken(1n, 0, 1);
            expect(decompression.amount).toBe(1n);

            const largeDecompression = createDecompressCtoken(
                BigInt('18446744073709551615'),
                0,
                1,
            );
            expect(largeDecompression.amount).toBe(
                BigInt('18446744073709551615'),
            );
        });
    });

    describe('createDecompressSpl', () => {
        it('should create decompression struct for SPL', () => {
            const decompression = createDecompressSpl(
                3000n, // amount
                0, // mintIndex
                1, // recipientIndex
                2, // poolAccountIndex
                0, // poolIndex
                253, // bump
                9, // decimals
            );

            expect(decompression.mode).toBe(COMPRESSION_MODE_DECOMPRESS);
            expect(decompression.amount).toBe(3000n);
            expect(decompression.mint).toBe(0);
            expect(decompression.sourceOrRecipient).toBe(1);
            expect(decompression.authority).toBe(0);
            expect(decompression.poolAccountIndex).toBe(2);
            expect(decompression.poolIndex).toBe(0);
            expect(decompression.bump).toBe(253);
            expect(decompression.decimals).toBe(9);
        });

        it('should handle different pool configurations', () => {
            const decompression = createDecompressSpl(
                1000n,
                0,
                1,
                5, // poolAccountIndex
                2, // poolIndex
                200, // bump
                6, // decimals
            );

            expect(decompression.poolAccountIndex).toBe(5);
            expect(decompression.poolIndex).toBe(2);
            expect(decompression.bump).toBe(200);
        });
    });

    describe('compression modes', () => {
        it('should have correct mode values', () => {
            expect(COMPRESSION_MODE_COMPRESS).toBe(0);
            expect(COMPRESSION_MODE_DECOMPRESS).toBe(1);
        });

        it('should set correct modes in factory functions', () => {
            const compress = createCompressSpl(100n, 0, 1, 2, 3, 0, 255, 9);
            expect(compress.mode).toBe(COMPRESSION_MODE_COMPRESS);

            const decompressCtoken = createDecompressCtoken(100n, 0, 1);
            expect(decompressCtoken.mode).toBe(COMPRESSION_MODE_DECOMPRESS);

            const decompressSpl = createDecompressSpl(100n, 0, 1, 2, 0, 255, 9);
            expect(decompressSpl.mode).toBe(COMPRESSION_MODE_DECOMPRESS);
        });
    });

    describe('encoding roundtrip integration', () => {
        it('should encode complex wrap instruction correctly', () => {
            const compressions = [
                createCompressSpl(1000n, 0, 2, 1, 4, 0, 255, 9),
                createDecompressCtoken(1000n, 0, 3, 6),
            ];

            const data: Transfer2InstructionData = {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 0,
                cpiContext: null,
                compressions,
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            };

            const encoded = encodeTransfer2InstructionData(data);

            // Should have discriminator
            expect(encoded.subarray(0, 1)).toEqual(TRANSFER2_DISCRIMINATOR);
            // Should be reasonable size (compressions + header)
            expect(encoded.length).toBeGreaterThan(30);
        });
    });
});
