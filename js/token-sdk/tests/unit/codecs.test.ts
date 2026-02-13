/**
 * Comprehensive codec roundtrip tests for Light Token SDK.
 *
 * Verifies that encoding then decoding produces the original data for all codecs.
 */

import { describe, it, expect } from 'vitest';

import {
    getCompressionCodec,
    getPackedMerkleContextCodec,
    getMultiInputTokenDataCodec,
    getMultiTokenOutputDataCodec,
    getCpiContextCodec,
    getCompressedProofCodec,
    getCompressibleExtensionDataCodec,
    getCreateAtaDataCodec,
    getAmountInstructionCodec,
    getCheckedInstructionCodec,
    getDiscriminatorOnlyCodec,
    encodeMaxTopUp,
    decodeMaxTopUp,
} from '../../src/codecs/index.js';

import { encodeTransfer2InstructionData } from '../../src/codecs/transfer2.js';

import type {
    Compression,
    PackedMerkleContext,
    MultiInputTokenDataWithContext,
    MultiTokenTransferOutputData,
    CompressedCpiContext,
    CompressedProof,
    CompressibleExtensionInstructionData,
    CreateAtaInstructionData,
    Transfer2InstructionData,
} from '../../src/codecs/types.js';

import type {
    AmountInstructionData,
    CheckedInstructionData,
    DiscriminatorOnlyData,
} from '../../src/codecs/instructions.js';

// ============================================================================
// 1. Compression codec roundtrip
// ============================================================================

describe('Compression codec', () => {
    it('roundtrip encodes and decodes all fields', () => {
        const codec = getCompressionCodec();
        const original: Compression = {
            mode: 2,
            amount: 1_000_000n,
            mint: 3,
            sourceOrRecipient: 5,
            authority: 7,
            poolAccountIndex: 9,
            poolIndex: 1,
            bump: 254,
            decimals: 9,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('handles zero amount', () => {
        const codec = getCompressionCodec();
        const original: Compression = {
            mode: 0,
            amount: 0n,
            mint: 0,
            sourceOrRecipient: 0,
            authority: 0,
            poolAccountIndex: 0,
            poolIndex: 0,
            bump: 0,
            decimals: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('handles max u64 amount', () => {
        const codec = getCompressionCodec();
        const original: Compression = {
            mode: 1,
            amount: 18446744073709551615n,
            mint: 255,
            sourceOrRecipient: 255,
            authority: 255,
            poolAccountIndex: 255,
            poolIndex: 255,
            bump: 255,
            decimals: 255,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 2. PackedMerkleContext codec roundtrip
// ============================================================================

describe('PackedMerkleContext codec', () => {
    it('roundtrip with proveByIndex true', () => {
        const codec = getPackedMerkleContextCodec();
        const original: PackedMerkleContext = {
            merkleTreePubkeyIndex: 1,
            queuePubkeyIndex: 2,
            leafIndex: 12345,
            proveByIndex: true,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip with proveByIndex false', () => {
        const codec = getPackedMerkleContextCodec();
        const original: PackedMerkleContext = {
            merkleTreePubkeyIndex: 0,
            queuePubkeyIndex: 0,
            leafIndex: 0,
            proveByIndex: false,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('handles max u32 leafIndex', () => {
        const codec = getPackedMerkleContextCodec();
        const original: PackedMerkleContext = {
            merkleTreePubkeyIndex: 255,
            queuePubkeyIndex: 255,
            leafIndex: 4294967295,
            proveByIndex: true,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 3. MultiInputTokenData codec roundtrip
// ============================================================================

describe('MultiInputTokenData codec', () => {
    it('roundtrip with delegate', () => {
        const codec = getMultiInputTokenDataCodec();
        const original: MultiInputTokenDataWithContext = {
            owner: 1,
            amount: 500_000n,
            hasDelegate: true,
            delegate: 3,
            mint: 2,
            version: 0,
            merkleContext: {
                merkleTreePubkeyIndex: 4,
                queuePubkeyIndex: 5,
                leafIndex: 999,
                proveByIndex: false,
            },
            rootIndex: 42,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip without delegate', () => {
        const codec = getMultiInputTokenDataCodec();
        const original: MultiInputTokenDataWithContext = {
            owner: 0,
            amount: 0n,
            hasDelegate: false,
            delegate: 0,
            mint: 0,
            version: 0,
            merkleContext: {
                merkleTreePubkeyIndex: 0,
                queuePubkeyIndex: 0,
                leafIndex: 0,
                proveByIndex: false,
            },
            rootIndex: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('handles max u16 rootIndex', () => {
        const codec = getMultiInputTokenDataCodec();
        const original: MultiInputTokenDataWithContext = {
            owner: 10,
            amount: 18446744073709551615n,
            hasDelegate: true,
            delegate: 20,
            mint: 30,
            version: 1,
            merkleContext: {
                merkleTreePubkeyIndex: 100,
                queuePubkeyIndex: 200,
                leafIndex: 4294967295,
                proveByIndex: true,
            },
            rootIndex: 65535,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 4. MultiTokenOutputData codec roundtrip
// ============================================================================

describe('MultiTokenOutputData codec', () => {
    it('roundtrip with standard values', () => {
        const codec = getMultiTokenOutputDataCodec();
        const original: MultiTokenTransferOutputData = {
            owner: 1,
            amount: 750_000n,
            hasDelegate: true,
            delegate: 2,
            mint: 3,
            version: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip without delegate', () => {
        const codec = getMultiTokenOutputDataCodec();
        const original: MultiTokenTransferOutputData = {
            owner: 5,
            amount: 100n,
            hasDelegate: false,
            delegate: 0,
            mint: 7,
            version: 1,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 5. CpiContext codec roundtrip
// ============================================================================

describe('CpiContext codec', () => {
    it('roundtrip with setContext true', () => {
        const codec = getCpiContextCodec();
        const original: CompressedCpiContext = {
            setContext: true,
            firstSetContext: true,
            cpiContextAccountIndex: 42,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip with setContext false', () => {
        const codec = getCpiContextCodec();
        const original: CompressedCpiContext = {
            setContext: false,
            firstSetContext: false,
            cpiContextAccountIndex: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 6. CompressedProof codec roundtrip
// ============================================================================

describe('CompressedProof codec', () => {
    it('roundtrip with populated proof data', () => {
        const codec = getCompressedProofCodec();
        const aBytes = new Uint8Array(32);
        aBytes.fill(0xaa);
        const bBytes = new Uint8Array(64);
        bBytes.fill(0xbb);
        const cBytes = new Uint8Array(32);
        cBytes.fill(0xcc);

        const original: CompressedProof = {
            a: aBytes,
            b: bBytes,
            c: cBytes,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(new Uint8Array(decoded.a)).toEqual(new Uint8Array(original.a));
        expect(new Uint8Array(decoded.b)).toEqual(new Uint8Array(original.b));
        expect(new Uint8Array(decoded.c)).toEqual(new Uint8Array(original.c));
    });

    it('verifies 32+64+32 byte sizes', () => {
        const codec = getCompressedProofCodec();
        const original: CompressedProof = {
            a: new Uint8Array(32).fill(1),
            b: new Uint8Array(64).fill(2),
            c: new Uint8Array(32).fill(3),
        };
        const encoded = codec.encode(original);

        // Total encoded size should be 32 + 64 + 32 = 128 bytes
        expect(encoded.length).toBe(128);
    });

    it('roundtrip with all-zero proof', () => {
        const codec = getCompressedProofCodec();
        const original: CompressedProof = {
            a: new Uint8Array(32),
            b: new Uint8Array(64),
            c: new Uint8Array(32),
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(new Uint8Array(decoded.a)).toEqual(new Uint8Array(32));
        expect(new Uint8Array(decoded.b)).toEqual(new Uint8Array(64));
        expect(new Uint8Array(decoded.c)).toEqual(new Uint8Array(32));
    });

    it('roundtrip with random-like proof data', () => {
        const codec = getCompressedProofCodec();
        const a = new Uint8Array(32);
        const b = new Uint8Array(64);
        const c = new Uint8Array(32);
        for (let i = 0; i < 32; i++) a[i] = i;
        for (let i = 0; i < 64; i++) b[i] = i % 256;
        for (let i = 0; i < 32; i++) c[i] = 255 - i;

        const original: CompressedProof = { a, b, c };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(new Uint8Array(decoded.a)).toEqual(new Uint8Array(original.a));
        expect(new Uint8Array(decoded.b)).toEqual(new Uint8Array(original.b));
        expect(new Uint8Array(decoded.c)).toEqual(new Uint8Array(original.c));
    });
});

// ============================================================================
// 7. CompressibleExtensionData codec roundtrip
// ============================================================================

describe('CompressibleExtensionData codec', () => {
    // Note: getOptionDecoder returns Option<T> ({ __option: 'Some'/'None' })
    // at runtime, while the types use T | null via `as unknown` casts.
    // For roundtrip tests, we verify that encode -> decode preserves semantics.

    it('roundtrip without compressToPubkey (null)', () => {
        const codec = getCompressibleExtensionDataCodec();
        const original: CompressibleExtensionInstructionData = {
            tokenAccountVersion: 0,
            rentPayment: 5,
            compressionOnly: 1,
            writeTopUp: 1000,
            compressToPubkey: null,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(decoded.tokenAccountVersion).toBe(
            original.tokenAccountVersion,
        );
        expect(decoded.rentPayment).toBe(original.rentPayment);
        expect(decoded.compressionOnly).toBe(original.compressionOnly);
        expect(decoded.writeTopUp).toBe(original.writeTopUp);

        // Decoded option field uses { __option: 'None' } at runtime
        const decodedPubkey = decoded.compressToPubkey as unknown;
        expect(decodedPubkey).toEqual({ __option: 'None' });
    });

    it('roundtrip with compressToPubkey', () => {
        const codec = getCompressibleExtensionDataCodec();
        const programId = new Uint8Array(32);
        programId.fill(0x11);
        const seed1 = new Uint8Array([1, 2, 3]);
        const seed2 = new Uint8Array([4, 5, 6, 7]);

        const original: CompressibleExtensionInstructionData = {
            tokenAccountVersion: 1,
            rentPayment: 10,
            compressionOnly: 0,
            writeTopUp: 50000,
            compressToPubkey: {
                bump: 254,
                programId: programId,
                seeds: [seed1, seed2],
            },
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(decoded.tokenAccountVersion).toBe(
            original.tokenAccountVersion,
        );
        expect(decoded.rentPayment).toBe(original.rentPayment);
        expect(decoded.compressionOnly).toBe(original.compressionOnly);
        expect(decoded.writeTopUp).toBe(original.writeTopUp);

        // Decoded option field uses { __option: 'Some', value: ... } at runtime
        const decodedPubkey = decoded.compressToPubkey as unknown as {
            __option: 'Some';
            value: {
                bump: number;
                programId: Uint8Array;
                seeds: Uint8Array[];
            };
        };
        expect(decodedPubkey.__option).toBe('Some');
        expect(decodedPubkey.value.bump).toBe(254);
        expect(new Uint8Array(decodedPubkey.value.programId)).toEqual(
            programId,
        );
        expect(decodedPubkey.value.seeds.length).toBe(2);
        expect(new Uint8Array(decodedPubkey.value.seeds[0])).toEqual(seed1);
        expect(new Uint8Array(decodedPubkey.value.seeds[1])).toEqual(seed2);
    });
});

// ============================================================================
// 8. CreateAtaData codec roundtrip
// ============================================================================

describe('CreateAtaData codec', () => {
    it('roundtrip without compressible config (null)', () => {
        const codec = getCreateAtaDataCodec();
        const original: CreateAtaInstructionData = {
            bump: 255,
            compressibleConfig: null,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(decoded.bump).toBe(original.bump);

        // Decoded option field uses { __option: 'None' } at runtime
        const decodedConfig = decoded.compressibleConfig as unknown;
        expect(decodedConfig).toEqual({ __option: 'None' });
    });

    it('roundtrip with compressible config', () => {
        const codec = getCreateAtaDataCodec();
        const original: CreateAtaInstructionData = {
            bump: 200,
            compressibleConfig: {
                tokenAccountVersion: 0,
                rentPayment: 3,
                compressionOnly: 0,
                writeTopUp: 0,
                compressToPubkey: null,
            },
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(decoded.bump).toBe(original.bump);

        // Outer option: { __option: 'Some', value: { ..., compressToPubkey: { __option: 'None' } } }
        const decodedConfig = decoded.compressibleConfig as unknown as {
            __option: 'Some';
            value: {
                tokenAccountVersion: number;
                rentPayment: number;
                compressionOnly: number;
                writeTopUp: number;
                compressToPubkey: { __option: 'None' };
            };
        };
        expect(decodedConfig.__option).toBe('Some');
        expect(decodedConfig.value.tokenAccountVersion).toBe(0);
        expect(decodedConfig.value.rentPayment).toBe(3);
        expect(decodedConfig.value.compressionOnly).toBe(0);
        expect(decodedConfig.value.writeTopUp).toBe(0);
        expect(decodedConfig.value.compressToPubkey).toEqual({
            __option: 'None',
        });
    });

    it('roundtrip with compressible config and compressToPubkey', () => {
        const codec = getCreateAtaDataCodec();
        const programId = new Uint8Array(32);
        programId.fill(0x42);

        const original: CreateAtaInstructionData = {
            bump: 128,
            compressibleConfig: {
                tokenAccountVersion: 1,
                rentPayment: 12,
                compressionOnly: 1,
                writeTopUp: 99999,
                compressToPubkey: {
                    bump: 253,
                    programId: programId,
                    seeds: [new Uint8Array([0xde, 0xad])],
                },
            },
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        expect(decoded.bump).toBe(128);

        const decodedConfig = decoded.compressibleConfig as unknown as {
            __option: 'Some';
            value: {
                tokenAccountVersion: number;
                rentPayment: number;
                compressionOnly: number;
                writeTopUp: number;
                compressToPubkey: {
                    __option: 'Some';
                    value: {
                        bump: number;
                        programId: Uint8Array;
                        seeds: Uint8Array[];
                    };
                };
            };
        };
        expect(decodedConfig.__option).toBe('Some');
        expect(decodedConfig.value.tokenAccountVersion).toBe(1);
        expect(decodedConfig.value.rentPayment).toBe(12);
        expect(decodedConfig.value.compressionOnly).toBe(1);
        expect(decodedConfig.value.writeTopUp).toBe(99999);
        expect(decodedConfig.value.compressToPubkey.__option).toBe('Some');
        expect(decodedConfig.value.compressToPubkey.value.bump).toBe(253);
        expect(
            new Uint8Array(decodedConfig.value.compressToPubkey.value.programId),
        ).toEqual(programId);
        expect(decodedConfig.value.compressToPubkey.value.seeds.length).toBe(1);
        expect(
            new Uint8Array(
                decodedConfig.value.compressToPubkey.value.seeds[0],
            ),
        ).toEqual(new Uint8Array([0xde, 0xad]));
    });
});

// ============================================================================
// 9. AmountInstructionData codec roundtrip
// ============================================================================

describe('AmountInstructionData codec', () => {
    it('roundtrip for transfer', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 3,
            amount: 1_000_000n,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for mint-to', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 7,
            amount: 5_000_000_000n,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for burn', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 8,
            amount: 250n,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for approve', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 4,
            amount: 999_999n,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('encoded size is 9 bytes (1 disc + 8 amount)', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 3,
            amount: 100n,
        };
        const encoded = codec.encode(original);
        expect(encoded.length).toBe(9);
    });
});

// ============================================================================
// 10. CheckedInstructionData codec roundtrip
// ============================================================================

describe('CheckedInstructionData codec', () => {
    it('roundtrip for transfer-checked', () => {
        const codec = getCheckedInstructionCodec();
        const original: CheckedInstructionData = {
            discriminator: 12,
            amount: 1_000_000n,
            decimals: 9,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for mint-to-checked', () => {
        const codec = getCheckedInstructionCodec();
        const original: CheckedInstructionData = {
            discriminator: 14,
            amount: 50_000n,
            decimals: 6,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for burn-checked', () => {
        const codec = getCheckedInstructionCodec();
        const original: CheckedInstructionData = {
            discriminator: 15,
            amount: 1n,
            decimals: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('encoded size is 10 bytes (1 disc + 8 amount + 1 decimals)', () => {
        const codec = getCheckedInstructionCodec();
        const original: CheckedInstructionData = {
            discriminator: 12,
            amount: 0n,
            decimals: 0,
        };
        const encoded = codec.encode(original);
        expect(encoded.length).toBe(10);
    });
});

// ============================================================================
// 11. DiscriminatorOnlyData codec roundtrip
// ============================================================================

describe('DiscriminatorOnlyData codec', () => {
    it('roundtrip for revoke', () => {
        const codec = getDiscriminatorOnlyCodec();
        const original: DiscriminatorOnlyData = { discriminator: 5 };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for freeze', () => {
        const codec = getDiscriminatorOnlyCodec();
        const original: DiscriminatorOnlyData = { discriminator: 10 };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for thaw', () => {
        const codec = getDiscriminatorOnlyCodec();
        const original: DiscriminatorOnlyData = { discriminator: 11 };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('roundtrip for close', () => {
        const codec = getDiscriminatorOnlyCodec();
        const original: DiscriminatorOnlyData = { discriminator: 9 };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });

    it('encoded size is 1 byte', () => {
        const codec = getDiscriminatorOnlyCodec();
        const original: DiscriminatorOnlyData = { discriminator: 5 };
        const encoded = codec.encode(original);
        expect(encoded.length).toBe(1);
    });
});

// ============================================================================
// 12. MaxTopUp encode/decode
// ============================================================================

describe('MaxTopUp encode/decode', () => {
    it('encodes undefined as empty bytes', () => {
        const encoded = encodeMaxTopUp(undefined);
        expect(encoded.length).toBe(0);
    });

    it('decodes undefined when no bytes remain', () => {
        const data = new Uint8Array([0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        // offset at 9 = data.length, so no bytes remain
        const result = decodeMaxTopUp(data, 9);
        expect(result).toBeUndefined();
    });

    it('roundtrip with a value', () => {
        const value = 1234;
        const encoded = encodeMaxTopUp(value);
        expect(encoded.length).toBe(2);

        // Place the encoded bytes into a buffer and decode at offset 0
        const decoded = decodeMaxTopUp(encoded, 0);
        expect(decoded).toBe(value);
    });

    it('roundtrip with zero', () => {
        const value = 0;
        const encoded = encodeMaxTopUp(value);
        expect(encoded.length).toBe(2);
        const decoded = decodeMaxTopUp(encoded, 0);
        expect(decoded).toBe(0);
    });

    it('roundtrip with max u16 value', () => {
        const value = 65535;
        const encoded = encodeMaxTopUp(value);
        expect(encoded.length).toBe(2);
        const decoded = decodeMaxTopUp(encoded, 0);
        expect(decoded).toBe(65535);
    });

    it('decodes from a specific offset within larger buffer', () => {
        // Build a buffer: [disc(1 byte), amount(8 bytes), maxTopUp(2 bytes)]
        const disc = new Uint8Array([3]);
        const amount = new Uint8Array(8);
        const topUpBytes = encodeMaxTopUp(500);
        const buffer = new Uint8Array(1 + 8 + 2);
        buffer.set(disc, 0);
        buffer.set(amount, 1);
        buffer.set(topUpBytes, 9);

        const decoded = decodeMaxTopUp(buffer, 9);
        expect(decoded).toBe(500);
    });
});

// ============================================================================
// 13. Edge cases
// ============================================================================

describe('Edge cases', () => {
    it('max u64 amount in Compression', () => {
        const codec = getCompressionCodec();
        const original: Compression = {
            mode: 0,
            amount: 18446744073709551615n,
            mint: 0,
            sourceOrRecipient: 0,
            authority: 0,
            poolAccountIndex: 0,
            poolIndex: 0,
            bump: 0,
            decimals: 0,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded.amount).toBe(18446744073709551615n);
    });

    it('max u64 amount in AmountInstructionData', () => {
        const codec = getAmountInstructionCodec();
        const original: AmountInstructionData = {
            discriminator: 3,
            amount: 18446744073709551615n,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded.amount).toBe(18446744073709551615n);
    });

    it('zero amount in all amount-bearing codecs', () => {
        const amountCodec = getAmountInstructionCodec();
        const amountData: AmountInstructionData = {
            discriminator: 3,
            amount: 0n,
        };
        expect(amountCodec.decode(amountCodec.encode(amountData)).amount).toBe(
            0n,
        );

        const checkedCodec = getCheckedInstructionCodec();
        const checkedData: CheckedInstructionData = {
            discriminator: 12,
            amount: 0n,
            decimals: 0,
        };
        expect(
            checkedCodec.decode(checkedCodec.encode(checkedData)).amount,
        ).toBe(0n);

        const compressionCodec = getCompressionCodec();
        const compressionData: Compression = {
            mode: 0,
            amount: 0n,
            mint: 0,
            sourceOrRecipient: 0,
            authority: 0,
            poolAccountIndex: 0,
            poolIndex: 0,
            bump: 0,
            decimals: 0,
        };
        expect(
            compressionCodec.decode(compressionCodec.encode(compressionData))
                .amount,
        ).toBe(0n);
    });

    it('all-zero CompressedProof', () => {
        const codec = getCompressedProofCodec();
        const original: CompressedProof = {
            a: new Uint8Array(32),
            b: new Uint8Array(64),
            c: new Uint8Array(32),
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        // All bytes should be zero
        expect(new Uint8Array(decoded.a).every((b) => b === 0)).toBe(true);
        expect(new Uint8Array(decoded.b).every((b) => b === 0)).toBe(true);
        expect(new Uint8Array(decoded.c).every((b) => b === 0)).toBe(true);
    });

    it('max u16 values in rootIndex and maxTopUp', () => {
        const inputCodec = getMultiInputTokenDataCodec();
        const inputData: MultiInputTokenDataWithContext = {
            owner: 0,
            amount: 0n,
            hasDelegate: false,
            delegate: 0,
            mint: 0,
            version: 0,
            merkleContext: {
                merkleTreePubkeyIndex: 0,
                queuePubkeyIndex: 0,
                leafIndex: 0,
                proveByIndex: false,
            },
            rootIndex: 65535,
        };
        const decoded = inputCodec.decode(inputCodec.encode(inputData));
        expect(decoded.rootIndex).toBe(65535);

        const topUpEncoded = encodeMaxTopUp(65535);
        const topUpDecoded = decodeMaxTopUp(topUpEncoded, 0);
        expect(topUpDecoded).toBe(65535);
    });

    it('max u8 values in all u8 fields', () => {
        const codec = getCompressionCodec();
        const original: Compression = {
            mode: 255,
            amount: 0n,
            mint: 255,
            sourceOrRecipient: 255,
            authority: 255,
            poolAccountIndex: 255,
            poolIndex: 255,
            bump: 255,
            decimals: 255,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded).toEqual(original);
    });
});

// ============================================================================
// 14. TLV encoding in encodeTransfer2InstructionData
// ============================================================================

describe('TLV encoding via encodeTransfer2InstructionData', () => {
    function makeMinimalTransfer2(
        overrides?: Partial<Transfer2InstructionData>,
    ): Transfer2InstructionData {
        return {
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
            ...overrides,
        };
    }

    it('null TLV produces [0] (Option::None) byte', () => {
        const data = makeMinimalTransfer2({
            inTlv: null,
            outTlv: null,
        });
        const encoded = encodeTransfer2InstructionData(data);

        // The last 2 bytes should be [0] [0] for inTlv=None, outTlv=None
        const lastTwo = encoded.slice(-2);
        expect(lastTwo[0]).toBe(0); // inTlv: None
        expect(lastTwo[1]).toBe(0); // outTlv: None
    });

    it('empty vec TLV produces [1, 0,0,0,0] (Option::Some(Vec[]))', () => {
        const data = makeMinimalTransfer2({
            inTlv: [],
            outTlv: null,
        });
        const encoded = encodeTransfer2InstructionData(data);

        // outTlv = None: last byte is 0
        expect(encoded[encoded.length - 1]).toBe(0);

        // inTlv = Some(Vec<>[]) = [1, 0,0,0,0]: 5 bytes before the last 1 byte
        const inTlvStart = encoded.length - 1 - 5;
        expect(encoded[inTlvStart]).toBe(1); // Option::Some
        expect(encoded[inTlvStart + 1]).toBe(0); // u32 length = 0
        expect(encoded[inTlvStart + 2]).toBe(0);
        expect(encoded[inTlvStart + 3]).toBe(0);
        expect(encoded[inTlvStart + 4]).toBe(0);
    });

    it('empty inner vec TLV produces correct bytes', () => {
        // inTlv = Some(Vec<Vec>[[]]) = [1, 1,0,0,0, 0,0,0,0]
        // This is 1 (Some) + 4 (outer len=1) + 4 (inner len=0) = 9 bytes
        const data = makeMinimalTransfer2({
            inTlv: [[]],
            outTlv: null,
        });
        const encoded = encodeTransfer2InstructionData(data);

        // outTlv = None: last byte is 0
        expect(encoded[encoded.length - 1]).toBe(0);

        // inTlv = Some(Vec[ Vec[] ]) = [1, 1,0,0,0, 0,0,0,0]: 9 bytes before last 1 byte
        const inTlvStart = encoded.length - 1 - 9;
        expect(encoded[inTlvStart]).toBe(1); // Option::Some
        // outer len = 1 (little-endian u32)
        expect(encoded[inTlvStart + 1]).toBe(1);
        expect(encoded[inTlvStart + 2]).toBe(0);
        expect(encoded[inTlvStart + 3]).toBe(0);
        expect(encoded[inTlvStart + 4]).toBe(0);
        // inner len = 0 (little-endian u32)
        expect(encoded[inTlvStart + 5]).toBe(0);
        expect(encoded[inTlvStart + 6]).toBe(0);
        expect(encoded[inTlvStart + 7]).toBe(0);
        expect(encoded[inTlvStart + 8]).toBe(0);
    });

    it('throws on non-empty inner vec', () => {
        const data = makeMinimalTransfer2({
            inTlv: [[{ type: 'CompressedOnly', data: {} }]] as never,
            outTlv: null,
        });
        expect(() => encodeTransfer2InstructionData(data)).toThrow(
            'TLV extension serialization is not yet implemented',
        );
    });

    it('both TLV fields null', () => {
        const data = makeMinimalTransfer2();
        const encoded = encodeTransfer2InstructionData(data);

        // Verify first byte is the discriminator (101 = TRANSFER2)
        expect(encoded[0]).toBe(101);

        // Last two bytes are both None (0)
        expect(encoded[encoded.length - 2]).toBe(0);
        expect(encoded[encoded.length - 1]).toBe(0);
    });

    it('encodes discriminator as first byte', () => {
        const data = makeMinimalTransfer2();
        const encoded = encodeTransfer2InstructionData(data);
        expect(encoded[0]).toBe(101);
    });
});
