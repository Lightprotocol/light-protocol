/**
 * Comprehensive codec roundtrip tests for Light Token SDK.
 *
 * Verifies that encoding then decoding produces the original data for all codecs.
 */

import { describe, it, expect } from 'vitest';
import { address, getAddressCodec } from '@solana/addresses';

import {
    getCompressionCodec,
    getPackedMerkleContextCodec,
    getMultiInputTokenDataCodec,
    getMultiTokenOutputDataCodec,
    getCpiContextCodec,
    getCompressedProofCodec,
    getCompressibleExtensionDataCodec,
    getCreateAtaDataCodec,
    getCreateTokenAccountDataCodec,
    encodeCreateTokenAccountInstructionData,
    getAmountInstructionCodec,
    getCheckedInstructionCodec,
    getDiscriminatorOnlyCodec,
    encodeMaxTopUp,
    decodeMaxTopUp,
} from '../../src/codecs/index.js';

import {
    encodeTransfer2InstructionData,
    encodeExtensionInstructionData,
    getTransfer2BaseEncoder,
    getTransfer2BaseDecoder,
} from '../../src/codecs/transfer2.js';

import {
    encodeMintActionInstructionData,
} from '../../src/codecs/mint-action.js';

import type {
    Compression,
    PackedMerkleContext,
    MultiInputTokenDataWithContext,
    MultiTokenTransferOutputData,
    CompressedCpiContext,
    CompressedProof,
    CompressibleExtensionInstructionData,
    CreateAtaInstructionData,
    CreateTokenAccountInstructionData,
    Transfer2InstructionData,
    ExtensionInstructionData,
    CompressionInfo,
    RentConfig,
    CompressedOnlyExtension,
    TokenMetadataExtension,
} from '../../src/codecs/types.js';

import type {
    MintActionInstructionData,
    MintMetadata,
    MintInstructionData,
    MintActionCpiContext,
    CreateMint,
} from '../../src/codecs/mint-action.js';

import type {
    AmountInstructionData,
    CheckedInstructionData,
    DiscriminatorOnlyData,
} from '../../src/codecs/instructions.js';

import { DISCRIMINATOR, EXTENSION_DISCRIMINANT } from '../../src/constants.js';

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
            compressibleConfig: null,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);

        // Decoded option field uses { __option: 'None' } at runtime
        const decodedConfig = decoded.compressibleConfig as unknown;
        expect(decodedConfig).toEqual({ __option: 'None' });
    });

    it('roundtrip with compressible config', () => {
        const codec = getCreateAtaDataCodec();
        const original: CreateAtaInstructionData = {
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
// 9. CreateTokenAccountData codec roundtrip
// ============================================================================

describe('CreateTokenAccountData codec', () => {
    const TEST_OWNER = address('11111111111111111111111111111111');

    it('roundtrip without compressible config (null)', () => {
        const codec = getCreateTokenAccountDataCodec();
        const original: CreateTokenAccountInstructionData = {
            owner: TEST_OWNER,
            compressibleConfig: null,
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded);
        expect(decoded.owner).toBe(TEST_OWNER);
        expect(decoded.compressibleConfig).toEqual({ __option: 'None' });
    });

    it('roundtrip with compressible config', () => {
        const codec = getCreateTokenAccountDataCodec();
        const original: CreateTokenAccountInstructionData = {
            owner: TEST_OWNER,
            compressibleConfig: {
                tokenAccountVersion: 3,
                rentPayment: 16,
                compressionOnly: 0,
                writeTopUp: 766,
                compressToPubkey: null,
            },
        };
        const encoded = codec.encode(original);
        const decoded = codec.decode(encoded) as unknown as {
            owner: string;
            compressibleConfig: {
                __option: 'Some';
                value: {
                    tokenAccountVersion: number;
                    rentPayment: number;
                    compressionOnly: number;
                    writeTopUp: number;
                };
            };
        };
        expect(decoded.owner).toBe(TEST_OWNER);
        expect(decoded.compressibleConfig.__option).toBe('Some');
        expect(decoded.compressibleConfig.value.tokenAccountVersion).toBe(3);
        expect(decoded.compressibleConfig.value.rentPayment).toBe(16);
        expect(decoded.compressibleConfig.value.compressionOnly).toBe(0);
        expect(decoded.compressibleConfig.value.writeTopUp).toBe(766);
    });

    it('encodeCreateTokenAccountInstructionData supports full and owner-only payloads', () => {
        const data: CreateTokenAccountInstructionData = {
            owner: TEST_OWNER,
            compressibleConfig: null,
        };
        const full = encodeCreateTokenAccountInstructionData(data);
        const ownerOnly = encodeCreateTokenAccountInstructionData(data, true);

        expect(full[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);
        expect(ownerOnly[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);
        expect(ownerOnly).toHaveLength(33);
        expect(ownerOnly.slice(1)).toEqual(
            new Uint8Array(getAddressCodec().encode(TEST_OWNER)),
        );
        expect(full.length).toBeGreaterThan(ownerOnly.length);
    });
});

// ============================================================================
// 10. AmountInstructionData codec roundtrip
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

    it('encodes CompressedOnly extension in TLV', () => {
        const data = makeMinimalTransfer2({
            inTlv: [[{
                type: 'CompressedOnly' as const,
                data: {
                    delegatedAmount: 0n,
                    withheldTransferFee: 0n,
                    isFrozen: false,
                    compressionIndex: 0,
                    isAta: true,
                    bump: 255,
                    ownerIndex: 1,
                },
            }]],
            outTlv: null,
        });
        const encoded = encodeTransfer2InstructionData(data);
        // Should not throw - TLV serialization is now implemented
        expect(encoded.length).toBeGreaterThan(0);
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

// ============================================================================
// 15. Transfer2 base data roundtrip via encoder/decoder
// ============================================================================

describe('Transfer2 base data roundtrip', () => {
    it('roundtrip with minimal data', () => {
        const encoder = getTransfer2BaseEncoder();
        const decoder = getTransfer2BaseDecoder();

        const original = {
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
        };
        const encoded = encoder.encode(original);
        const decoded = decoder.decode(encoded);

        expect(decoded.withTransactionHash).toBe(false);
        expect(decoded.outputQueue).toBe(0);
        expect(decoded.maxTopUp).toBe(0);
        expect(decoded.inTokenData).toHaveLength(0);
        expect(decoded.outTokenData).toHaveLength(0);
    });

    it('roundtrip with populated fields', () => {
        const encoder = getTransfer2BaseEncoder();
        const decoder = getTransfer2BaseDecoder();

        const original = {
            withTransactionHash: true,
            withLamportsChangeAccountMerkleTreeIndex: true,
            lamportsChangeAccountMerkleTreeIndex: 5,
            lamportsChangeAccountOwnerIndex: 3,
            outputQueue: 2,
            maxTopUp: 1000,
            cpiContext: { setContext: true, firstSetContext: false },
            compressions: null,
            proof: null,
            inTokenData: [
                {
                    owner: 1,
                    amount: 5000n,
                    hasDelegate: false,
                    delegate: 0,
                    mint: 2,
                    version: 3,
                    merkleContext: {
                        merkleTreePubkeyIndex: 4,
                        queuePubkeyIndex: 5,
                        leafIndex: 100,
                        proveByIndex: true,
                    },
                    rootIndex: 42,
                },
            ],
            outTokenData: [
                {
                    owner: 6,
                    amount: 3000n,
                    hasDelegate: false,
                    delegate: 0,
                    mint: 2,
                    version: 3,
                },
                {
                    owner: 1,
                    amount: 2000n,
                    hasDelegate: false,
                    delegate: 0,
                    mint: 2,
                    version: 3,
                },
            ],
            inLamports: null,
            outLamports: null,
        };
        const encoded = encoder.encode(original);
        const decoded = decoder.decode(encoded);

        expect(decoded.withTransactionHash).toBe(true);
        expect(decoded.lamportsChangeAccountMerkleTreeIndex).toBe(5);
        expect(decoded.outputQueue).toBe(2);
        expect(decoded.maxTopUp).toBe(1000);
        expect(decoded.inTokenData).toHaveLength(1);
        expect(decoded.inTokenData[0].amount).toBe(5000n);
        expect(decoded.inTokenData[0].rootIndex).toBe(42);
        expect(decoded.outTokenData).toHaveLength(2);
        expect(decoded.outTokenData[0].amount).toBe(3000n);
        expect(decoded.outTokenData[1].amount).toBe(2000n);
    });

    it('roundtrip with lamports fields', () => {
        const encoder = getTransfer2BaseEncoder();
        const decoder = getTransfer2BaseDecoder();

        const original = {
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
        };
        const encoded = encoder.encode(original);
        const decoded = decoder.decode(encoded);

        // Option<Vec<u64>> fields
        const inLamports = decoded.inLamports as unknown as {
            __option: string;
            value?: bigint[];
        };
        expect(inLamports.__option).toBe('Some');
        expect(inLamports.value).toHaveLength(2);
        expect(inLamports.value![0]).toBe(1000000n);
        expect(inLamports.value![1]).toBe(2000000n);
    });

    it('roundtrip with compression operations', () => {
        const encoder = getTransfer2BaseEncoder();
        const decoder = getTransfer2BaseDecoder();

        const original = {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: 0,
            outputQueue: 0,
            maxTopUp: 0,
            cpiContext: null,
            compressions: [
                {
                    mode: 0,
                    amount: 1000000n,
                    mint: 1,
                    sourceOrRecipient: 2,
                    authority: 3,
                    poolAccountIndex: 4,
                    poolIndex: 0,
                    bump: 255,
                    decimals: 9,
                },
            ],
            proof: null,
            inTokenData: [],
            outTokenData: [],
            inLamports: null,
            outLamports: null,
        };
        const encoded = encoder.encode(original);
        const decoded = decoder.decode(encoded);

        const compressions = decoded.compressions as unknown as {
            __option: string;
            value?: Compression[];
        };
        expect(compressions.__option).toBe('Some');
        expect(compressions.value).toHaveLength(1);
        expect(compressions.value![0].amount).toBe(1000000n);
        expect(compressions.value![0].bump).toBe(255);
    });
});

// ============================================================================
// 16. Extension encoding byte-level tests
// ============================================================================

describe('Extension encoding byte-level', () => {
    it('PausableAccount encodes as single discriminant byte [27]', () => {
        const ext: ExtensionInstructionData = { type: 'PausableAccount' };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded).toEqual(new Uint8Array([EXTENSION_DISCRIMINANT.PAUSABLE_ACCOUNT]));
        expect(encoded.length).toBe(1);
    });

    it('PermanentDelegateAccount encodes as single discriminant byte [28]', () => {
        const ext: ExtensionInstructionData = { type: 'PermanentDelegateAccount' };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded).toEqual(new Uint8Array([EXTENSION_DISCRIMINANT.PERMANENT_DELEGATE_ACCOUNT]));
        expect(encoded.length).toBe(1);
    });

    it('TransferFeeAccount encodes as single discriminant byte [29]', () => {
        const ext: ExtensionInstructionData = { type: 'TransferFeeAccount' };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded).toEqual(new Uint8Array([EXTENSION_DISCRIMINANT.TRANSFER_FEE_ACCOUNT]));
        expect(encoded.length).toBe(1);
    });

    it('TransferHookAccount encodes as single discriminant byte [30]', () => {
        const ext: ExtensionInstructionData = { type: 'TransferHookAccount' };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded).toEqual(new Uint8Array([EXTENSION_DISCRIMINANT.TRANSFER_HOOK_ACCOUNT]));
        expect(encoded.length).toBe(1);
    });

    it('CompressedOnly encodes discriminant [31] + 20 bytes of data', () => {
        const ext: ExtensionInstructionData = {
            type: 'CompressedOnly',
            data: {
                delegatedAmount: 1000n,
                withheldTransferFee: 500n,
                isFrozen: true,
                compressionIndex: 42,
                isAta: false,
                bump: 253,
                ownerIndex: 7,
            },
        };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded[0]).toBe(EXTENSION_DISCRIMINANT.COMPRESSED_ONLY);
        // CompressedOnly: u64(8) + u64(8) + bool(1) + u8(1) + bool(1) + u8(1) + u8(1) = 21 bytes + 1 disc
        expect(encoded.length).toBe(22);

        // Verify delegatedAmount (LE u64 at offset 1)
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getBigUint64(1, true)).toBe(1000n);
        // Verify withheldTransferFee (LE u64 at offset 9)
        expect(view.getBigUint64(9, true)).toBe(500n);
        // isFrozen (bool at offset 17)
        expect(encoded[17]).toBe(1);
        // compressionIndex (u8 at offset 18)
        expect(encoded[18]).toBe(42);
        // isAta (bool at offset 19)
        expect(encoded[19]).toBe(0);
        // bump (u8 at offset 20)
        expect(encoded[20]).toBe(253);
        // ownerIndex (u8 at offset 21)
        expect(encoded[21]).toBe(7);
    });

    it('Compressible encodes discriminant [32] + CompressionInfo bytes', () => {
        const compressionAuthority = new Uint8Array(32).fill(0xaa);
        const rentSponsor = new Uint8Array(32).fill(0xbb);

        const ext: ExtensionInstructionData = {
            type: 'Compressible',
            data: {
                configAccountVersion: 1,
                compressToPubkey: 2,
                accountVersion: 0,
                lamportsPerWrite: 5000,
                compressionAuthority,
                rentSponsor,
                lastClaimedSlot: 42n,
                rentExemptionPaid: 1000,
                reserved: 0,
                rentConfig: {
                    baseRent: 100,
                    compressionCost: 200,
                    lamportsPerBytePerEpoch: 3,
                    maxFundedEpochs: 10,
                    maxTopUp: 500,
                },
            },
        };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded[0]).toBe(EXTENSION_DISCRIMINANT.COMPRESSIBLE);
        // CompressionInfo: u16(2) + u8(1) + u8(1) + u32(4) + pubkey(32) + pubkey(32)
        //   + u64(8) + u32(4) + u32(4) + RentConfig(2+2+1+1+2=8) = 96 bytes + 1 disc
        expect(encoded.length).toBe(97);

        const view = new DataView(encoded.buffer, encoded.byteOffset);
        // configAccountVersion (u16 at offset 1)
        expect(view.getUint16(1, true)).toBe(1);
        // compressToPubkey (u8 at offset 3)
        expect(encoded[3]).toBe(2);
        // accountVersion (u8 at offset 4)
        expect(encoded[4]).toBe(0);
        // lamportsPerWrite (u32 at offset 5)
        expect(view.getUint32(5, true)).toBe(5000);
        // compressionAuthority (32 bytes at offset 9)
        expect(encoded.slice(9, 41).every((b) => b === 0xaa)).toBe(true);
        // rentSponsor (32 bytes at offset 41)
        expect(encoded.slice(41, 73).every((b) => b === 0xbb)).toBe(true);
        // lastClaimedSlot (u64 at offset 73)
        expect(view.getBigUint64(73, true)).toBe(42n);
        // rentExemptionPaid (u32 at offset 81)
        expect(view.getUint32(81, true)).toBe(1000);
        // reserved (u32 at offset 85)
        expect(view.getUint32(85, true)).toBe(0);
        // RentConfig.baseRent (u16 at offset 89)
        expect(view.getUint16(89, true)).toBe(100);
        // RentConfig.compressionCost (u16 at offset 91)
        expect(view.getUint16(91, true)).toBe(200);
        // RentConfig.lamportsPerBytePerEpoch (u8 at offset 93)
        expect(encoded[93]).toBe(3);
        // RentConfig.maxFundedEpochs (u8 at offset 94)
        expect(encoded[94]).toBe(10);
        // RentConfig.maxTopUp (u16 at offset 95)
        expect(view.getUint16(95, true)).toBe(500);
    });

    it('TokenMetadata encodes discriminant [19] + metadata fields', () => {
        const name = new TextEncoder().encode('TestToken');
        const symbol = new TextEncoder().encode('TT');
        const uri = new TextEncoder().encode('https://example.com');

        const ext: ExtensionInstructionData = {
            type: 'TokenMetadata',
            data: {
                updateAuthority: null,
                name,
                symbol,
                uri,
                additionalMetadata: null,
            },
        };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded[0]).toBe(EXTENSION_DISCRIMINANT.TOKEN_METADATA);

        // After disc: Option<Pubkey>=None(1) + Vec<u8> name (4+9)
        //   + Vec<u8> symbol (4+2) + Vec<u8> uri (4+19) + Option<Vec>=None(1)
        // = 1 + 1 + 13 + 6 + 23 + 1 = 45
        expect(encoded.length).toBe(45);

        // updateAuthority = None
        expect(encoded[1]).toBe(0);
        // name Vec len
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(2, true)).toBe(9);
        // name content
        const decodedName = new TextDecoder().decode(encoded.slice(6, 15));
        expect(decodedName).toBe('TestToken');
    });

    it('TokenMetadata with updateAuthority and additionalMetadata', () => {
        const name = new TextEncoder().encode('A');
        const symbol = new TextEncoder().encode('B');
        const uri = new TextEncoder().encode('C');
        // Use a valid base58 address for updateAuthority
        const updateAuthority = '11111111111111111111111111111111';

        const ext: ExtensionInstructionData = {
            type: 'TokenMetadata',
            data: {
                updateAuthority: updateAuthority as any,
                name,
                symbol,
                uri,
                additionalMetadata: [
                    {
                        key: new TextEncoder().encode('key1'),
                        value: new TextEncoder().encode('val1'),
                    },
                ],
            },
        };
        const encoded = encodeExtensionInstructionData(ext);
        expect(encoded[0]).toBe(EXTENSION_DISCRIMINANT.TOKEN_METADATA);

        // updateAuthority = Some (offset 1)
        expect(encoded[1]).toBe(1);
        // After updateAuthority (32 bytes) at offset 2..34
        // name Vec: 4+1 at offset 34
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(34, true)).toBe(1); // name len
        // additionalMetadata = Some
        // Find additionalMetadata option byte - it's after disc(1) + option(1) + pubkey(32)
        //   + name(4+1) + symbol(4+1) + uri(4+1) = 49
        expect(encoded[49]).toBe(1); // Some
        // Vec len = 1 (4 bytes)
        expect(view.getUint32(50, true)).toBe(1);
    });
});

// ============================================================================
// 17. MintAction codec byte-level tests
// ============================================================================

describe('MintAction codec encoding', () => {
    function makeMinimalMintAction(
        overrides?: Partial<MintActionInstructionData>,
    ): MintActionInstructionData {
        return {
            leafIndex: 0,
            proveByIndex: false,
            rootIndex: 0,
            maxTopUp: 0,
            createMint: null,
            actions: [],
            proof: null,
            cpiContext: null,
            mint: null,
            ...overrides,
        };
    }

    it('starts with MINT_ACTION discriminator (103)', () => {
        const data = makeMinimalMintAction();
        const encoded = encodeMintActionInstructionData(data);
        expect(encoded[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        expect(encoded[0]).toBe(103);
    });

    it('encodes fixed header fields correctly', () => {
        const data = makeMinimalMintAction({
            leafIndex: 12345,
            proveByIndex: true,
            rootIndex: 42,
            maxTopUp: 1000,
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        // disc (1) + leafIndex (u32 at offset 1)
        expect(view.getUint32(1, true)).toBe(12345);
        // proveByIndex (bool at offset 5)
        expect(encoded[5]).toBe(1);
        // rootIndex (u16 at offset 6)
        expect(view.getUint16(6, true)).toBe(42);
        // maxTopUp (u16 at offset 8)
        expect(view.getUint16(8, true)).toBe(1000);
    });

    it('encodes null createMint as Option::None [0]', () => {
        const data = makeMinimalMintAction();
        const encoded = encodeMintActionInstructionData(data);
        // After fixed header: disc(1) + u32(4) + bool(1) + u16(2) + u16(2) = 10
        expect(encoded[10]).toBe(0); // createMint = None
    });

    it('encodes createMint as Option::Some with tree and root indices', () => {
        const addressTrees = new Uint8Array([1, 2, 3, 4]);
        const data = makeMinimalMintAction({
            createMint: {
                readOnlyAddressTrees: addressTrees,
                readOnlyAddressTreeRootIndices: [100, 200, 300, 400],
            },
        });
        const encoded = encodeMintActionInstructionData(data);
        // createMint = Some at offset 10
        expect(encoded[10]).toBe(1);
        // readOnlyAddressTrees (4 bytes at offset 11)
        expect(encoded[11]).toBe(1);
        expect(encoded[12]).toBe(2);
        expect(encoded[13]).toBe(3);
        expect(encoded[14]).toBe(4);
        // 4 x u16 root indices at offset 15
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint16(15, true)).toBe(100);
        expect(view.getUint16(17, true)).toBe(200);
        expect(view.getUint16(19, true)).toBe(300);
        expect(view.getUint16(21, true)).toBe(400);
    });

    it('encodes empty actions vec as [0,0,0,0]', () => {
        const data = makeMinimalMintAction();
        const encoded = encodeMintActionInstructionData(data);
        // After None createMint: offset 11 = actions vec length (u32)
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(11, true)).toBe(0);
    });

    it('encodes MintToCompressed action (discriminant 0)', () => {
        const recipient = new Uint8Array(32).fill(0xab);
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'MintToCompressed',
                    tokenAccountVersion: 3,
                    recipients: [{ recipient, amount: 1000000n }],
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        // actions vec len = 1 at offset 11
        expect(view.getUint32(11, true)).toBe(1);
        // action disc = 0 at offset 15
        expect(encoded[15]).toBe(0);
        // tokenAccountVersion = 3 at offset 16
        expect(encoded[16]).toBe(3);
        // recipients vec len = 1 at offset 17
        expect(view.getUint32(17, true)).toBe(1);
        // recipient pubkey (32 bytes at offset 21)
        expect(encoded[21]).toBe(0xab);
        expect(encoded[52]).toBe(0xab);
        // amount (u64 at offset 53)
        expect(view.getBigUint64(53, true)).toBe(1000000n);
    });

    it('encodes MintTo action (discriminant 3)', () => {
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'MintTo',
                    accountIndex: 5,
                    amount: 999n,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        expect(view.getUint32(11, true)).toBe(1);
        expect(encoded[15]).toBe(3); // MintTo disc
        expect(encoded[16]).toBe(5); // accountIndex
        expect(view.getBigUint64(17, true)).toBe(999n);
    });

    it('encodes UpdateMintAuthority action (discriminant 1)', () => {
        const newAuth = new Uint8Array(32).fill(0xcc);
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'UpdateMintAuthority',
                    newAuthority: newAuth,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);

        expect(encoded[15]).toBe(1); // UpdateMintAuthority disc
        expect(encoded[16]).toBe(1); // Option::Some
        expect(encoded[17]).toBe(0xcc); // first byte of authority
    });

    it('encodes UpdateMintAuthority with null (revoke)', () => {
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'UpdateMintAuthority',
                    newAuthority: null,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);

        expect(encoded[15]).toBe(1); // UpdateMintAuthority disc
        expect(encoded[16]).toBe(0); // Option::None
    });

    it('encodes UpdateFreezeAuthority action (discriminant 2)', () => {
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'UpdateFreezeAuthority',
                    newAuthority: null,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);

        expect(encoded[15]).toBe(2); // UpdateFreezeAuthority disc
        expect(encoded[16]).toBe(0); // None
    });

    it('encodes UpdateMetadataField action (discriminant 4)', () => {
        const key = new TextEncoder().encode('name');
        const value = new TextEncoder().encode('NewName');
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'UpdateMetadataField',
                    extensionIndex: 0,
                    fieldType: 0, // Name
                    key,
                    value,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        expect(encoded[15]).toBe(4); // UpdateMetadataField disc
        expect(encoded[16]).toBe(0); // extensionIndex
        expect(encoded[17]).toBe(0); // fieldType (Name)
        // key Vec: len=4 at offset 18
        expect(view.getUint32(18, true)).toBe(4);
        // key content at offset 22
        expect(new TextDecoder().decode(encoded.slice(22, 26))).toBe('name');
        // value Vec: len=7 at offset 26
        expect(view.getUint32(26, true)).toBe(7);
        expect(new TextDecoder().decode(encoded.slice(30, 37))).toBe('NewName');
    });

    it('encodes UpdateMetadataAuthority action (discriminant 5)', () => {
        const newAuth = new Uint8Array(32).fill(0xdd);
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'UpdateMetadataAuthority',
                    extensionIndex: 2,
                    newAuthority: newAuth,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);

        expect(encoded[15]).toBe(5); // disc
        expect(encoded[16]).toBe(2); // extensionIndex
        expect(encoded[17]).toBe(0xdd); // first byte of authority
    });

    it('encodes RemoveMetadataKey action (discriminant 6)', () => {
        const key = new TextEncoder().encode('key1');
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'RemoveMetadataKey',
                    extensionIndex: 1,
                    key,
                    idempotent: 1,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        expect(encoded[15]).toBe(6); // disc
        expect(encoded[16]).toBe(1); // extensionIndex
        expect(view.getUint32(17, true)).toBe(4); // key Vec len
        expect(new TextDecoder().decode(encoded.slice(21, 25))).toBe('key1');
        expect(encoded[25]).toBe(1); // idempotent
    });

    it('encodes DecompressMint action (discriminant 7)', () => {
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'DecompressMint',
                    rentPayment: 5,
                    writeTopUp: 10000,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        expect(encoded[15]).toBe(7); // disc
        expect(encoded[16]).toBe(5); // rentPayment (u8)
        expect(view.getUint32(17, true)).toBe(10000); // writeTopUp (u32)
    });

    it('encodes CompressAndCloseMint action (discriminant 8)', () => {
        const data = makeMinimalMintAction({
            actions: [
                {
                    type: 'CompressAndCloseMint',
                    idempotent: 1,
                },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);

        expect(encoded[15]).toBe(8); // disc
        expect(encoded[16]).toBe(1); // idempotent
    });

    it('encodes multiple actions sequentially', () => {
        const data = makeMinimalMintAction({
            actions: [
                { type: 'CompressAndCloseMint', idempotent: 0 },
                { type: 'MintTo', accountIndex: 1, amount: 100n },
            ],
        });
        const encoded = encodeMintActionInstructionData(data);
        const view = new DataView(encoded.buffer, encoded.byteOffset);

        // actions vec len = 2
        expect(view.getUint32(11, true)).toBe(2);
        // First action: CompressAndCloseMint
        expect(encoded[15]).toBe(8);
        expect(encoded[16]).toBe(0);
        // Second action: MintTo at offset 17
        expect(encoded[17]).toBe(3);
    });

    it('encodes MintMetadata as fixed 67 bytes', () => {
        const mint = new Uint8Array(32).fill(0x11);
        const mintSigner = new Uint8Array(32).fill(0x22);

        const metadata: MintMetadata = {
            version: 1,
            mintDecompressed: true,
            mint,
            mintSigner,
            bump: 254,
        };

        const mintData: MintInstructionData = {
            supply: 1000000n,
            decimals: 9,
            metadata,
            mintAuthority: null,
            freezeAuthority: null,
            extensions: null,
        };

        const data = makeMinimalMintAction({ mint: mintData });
        const encoded = encodeMintActionInstructionData(data);

        // Find the mint data section. After:
        // disc(1) + header(9) + createMint None(1) + actions Vec(4) + proof None(1) + cpiContext None(1)
        // = 17 bytes, then mint = Some(1) = offset 17
        // But wait: actions is empty so no action bytes. Let me calculate:
        // disc(1) + leafIndex(4) + proveByIndex(1) + rootIndex(2) + maxTopUp(2) = 10
        // + createMint None(1) = 11
        // + actions vec len(4) + 0 action bytes = 15
        // + proof None(1) = 16
        // + cpiContext None(1) = 17
        // + mint Some(1) = 18
        // + supply(8) = offset 18..26
        // + decimals(1) = offset 26
        // + MintMetadata starts at offset 27

        const view = new DataView(encoded.buffer, encoded.byteOffset);

        // mint option = Some at offset 17
        expect(encoded[17]).toBe(1);
        // supply (u64)
        expect(view.getBigUint64(18, true)).toBe(1000000n);
        // decimals
        expect(encoded[26]).toBe(9);

        // MintMetadata at offset 27:
        // version (u8)
        expect(encoded[27]).toBe(1);
        // mintDecompressed (bool)
        expect(encoded[28]).toBe(1);
        // mint pubkey (32 bytes)
        expect(encoded[29]).toBe(0x11);
        expect(encoded[60]).toBe(0x11);
        // mintSigner (32 bytes starting at offset 61)
        expect(encoded[61]).toBe(0x22);
        expect(encoded[92]).toBe(0x22);
        // bump (u8 at offset 93)
        expect(encoded[93]).toBe(254);

        // Total MintMetadata = 1 + 1 + 32 + 32 + 1 = 67 bytes
        const metadataSlice = encoded.slice(27, 94);
        expect(metadataSlice.length).toBe(67);
    });

    it('encodes MintInstructionData with authorities and extensions', () => {
        const mint = new Uint8Array(32).fill(0);
        const mintSigner = new Uint8Array(32).fill(0);
        const mintAuth = new Uint8Array(32).fill(0xaa);
        const freezeAuth = new Uint8Array(32).fill(0xbb);

        const mintData: MintInstructionData = {
            supply: 0n,
            decimals: 6,
            metadata: {
                version: 0,
                mintDecompressed: false,
                mint,
                mintSigner,
                bump: 0,
            },
            mintAuthority: mintAuth,
            freezeAuthority: freezeAuth,
            extensions: [{ type: 'PausableAccount' }],
        };

        const data = makeMinimalMintAction({ mint: mintData });
        const encoded = encodeMintActionInstructionData(data);

        // After MintMetadata (67 bytes starting at offset 27, ends at offset 94):
        // mintAuthority = Some(1) + 32 bytes at offset 94
        expect(encoded[94]).toBe(1); // Some
        expect(encoded[95]).toBe(0xaa); // first byte
        // freezeAuthority = Some(1) + 32 bytes at offset 127
        expect(encoded[127]).toBe(1); // Some
        expect(encoded[128]).toBe(0xbb); // first byte
        // extensions = Some(1) + Vec len(4) + PausableAccount disc(1) at offset 160
        expect(encoded[160]).toBe(1); // Some
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(161, true)).toBe(1); // Vec len
        expect(encoded[165]).toBe(27); // PausableAccount discriminant
    });

    it('encodes MintActionCpiContext with all fields', () => {
        const addressTreePubkey = new Uint8Array(32).fill(0xee);
        const readOnlyAddressTrees = new Uint8Array([10, 20, 30, 40]);

        const cpiCtx: MintActionCpiContext = {
            setContext: true,
            firstSetContext: false,
            inTreeIndex: 1,
            inQueueIndex: 2,
            outQueueIndex: 3,
            tokenOutQueueIndex: 4,
            assignedAccountIndex: 5,
            readOnlyAddressTrees,
            addressTreePubkey,
        };

        const data = makeMinimalMintAction({ cpiContext: cpiCtx });
        const encoded = encodeMintActionInstructionData(data);

        // After disc(1) + header(9) + createMint None(1) + actions(4) + proof None(1) = 16
        // cpiContext Some(1) at offset 16
        expect(encoded[16]).toBe(1);
        // setContext (bool at offset 17)
        expect(encoded[17]).toBe(1);
        // firstSetContext (bool at offset 18)
        expect(encoded[18]).toBe(0);
        // inTreeIndex (u8 at 19)
        expect(encoded[19]).toBe(1);
        // inQueueIndex (u8 at 20)
        expect(encoded[20]).toBe(2);
        // outQueueIndex (u8 at 21)
        expect(encoded[21]).toBe(3);
        // tokenOutQueueIndex (u8 at 22)
        expect(encoded[22]).toBe(4);
        // assignedAccountIndex (u8 at 23)
        expect(encoded[23]).toBe(5);
        // readOnlyAddressTrees (4 bytes at 24)
        expect(encoded[24]).toBe(10);
        expect(encoded[25]).toBe(20);
        expect(encoded[26]).toBe(30);
        expect(encoded[27]).toBe(40);
        // addressTreePubkey (32 bytes at 28)
        expect(encoded[28]).toBe(0xee);
    });

    it('encodes proof via CompressedProof encoder', () => {
        const proof = {
            a: new Uint8Array(32).fill(0x11),
            b: new Uint8Array(64).fill(0x22),
            c: new Uint8Array(32).fill(0x33),
        };

        const data = makeMinimalMintAction({ proof });
        const encoded = encodeMintActionInstructionData(data);

        // proof at offset 15 (after disc(1) + header(9) + None(1) + actionsVec(4))
        expect(encoded[15]).toBe(1); // Some
        // proof.a (32 bytes at offset 16)
        expect(encoded[16]).toBe(0x11);
        // proof.b (64 bytes at offset 48)
        expect(encoded[48]).toBe(0x22);
        // proof.c (32 bytes at offset 112)
        expect(encoded[112]).toBe(0x33);
        // Total proof = 128 bytes, offset 16..144
    });
});

// ============================================================================
// 18. TLV content verification (byte-level extension data in Transfer2)
// ============================================================================

describe('TLV content verification', () => {
    function makeMinimalTransfer2WithTlv(
        inTlv: ExtensionInstructionData[][] | null,
        outTlv: ExtensionInstructionData[][] | null,
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
            inTlv,
            outTlv,
        };
    }

    it('multiple extensions per account are encoded sequentially', () => {
        const data = makeMinimalTransfer2WithTlv(
            [[
                { type: 'PausableAccount' },
                { type: 'PermanentDelegateAccount' },
                { type: 'TransferFeeAccount' },
            ]],
            null,
        );
        const encoded = encodeTransfer2InstructionData(data);

        // outTlv = None: last byte is 0
        expect(encoded[encoded.length - 1]).toBe(0);

        // inTlv structure: Some(1) + outer_len=1(4) + inner_len=3(4) + ext1(1) + ext2(1) + ext3(1)
        // = 12 bytes before the last None byte
        const inTlvStart = encoded.length - 1 - 12;
        expect(encoded[inTlvStart]).toBe(1); // Some
        // outer len = 1
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(inTlvStart + 1, true)).toBe(1);
        // inner len = 3
        expect(view.getUint32(inTlvStart + 5, true)).toBe(3);
        // extensions
        expect(encoded[inTlvStart + 9]).toBe(27); // PausableAccount
        expect(encoded[inTlvStart + 10]).toBe(28); // PermanentDelegateAccount
        expect(encoded[inTlvStart + 11]).toBe(29); // TransferFeeAccount
    });

    it('multiple accounts with different extensions', () => {
        const data = makeMinimalTransfer2WithTlv(
            [
                [{ type: 'PausableAccount' }],
                [{ type: 'TransferHookAccount' }],
            ],
            null,
        );
        const encoded = encodeTransfer2InstructionData(data);

        expect(encoded[encoded.length - 1]).toBe(0); // outTlv None

        // inTlv: Some(1) + outer_len=2(4) + inner1_len=1(4) + ext1(1) + inner2_len=1(4) + ext2(1)
        // = 15 bytes before last None byte
        const inTlvStart = encoded.length - 1 - 15;
        expect(encoded[inTlvStart]).toBe(1); // Some
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(inTlvStart + 1, true)).toBe(2); // 2 accounts
        // First inner vec
        expect(view.getUint32(inTlvStart + 5, true)).toBe(1);
        expect(encoded[inTlvStart + 9]).toBe(27); // PausableAccount
        // Second inner vec
        expect(view.getUint32(inTlvStart + 10, true)).toBe(1);
        expect(encoded[inTlvStart + 14]).toBe(30); // TransferHookAccount
    });

    it('both inTlv and outTlv populated', () => {
        const data = makeMinimalTransfer2WithTlv(
            [[{ type: 'PausableAccount' }]],
            [[{ type: 'TransferFeeAccount' }]],
        );
        const encoded = encodeTransfer2InstructionData(data);

        // outTlv at the end: Some(1) + outer_len=1(4) + inner_len=1(4) + ext(1) = 10 bytes
        const outTlvStart = encoded.length - 10;
        expect(encoded[outTlvStart]).toBe(1); // Some
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getUint32(outTlvStart + 1, true)).toBe(1);
        expect(view.getUint32(outTlvStart + 5, true)).toBe(1);
        expect(encoded[outTlvStart + 9]).toBe(29); // TransferFeeAccount

        // inTlv before outTlv: also 10 bytes
        const inTlvStart = outTlvStart - 10;
        expect(encoded[inTlvStart]).toBe(1); // Some
        expect(view.getUint32(inTlvStart + 1, true)).toBe(1);
        expect(view.getUint32(inTlvStart + 5, true)).toBe(1);
        expect(encoded[inTlvStart + 9]).toBe(27); // PausableAccount
    });

    it('CompressedOnly extension data bytes are correct in TLV', () => {
        const data = makeMinimalTransfer2WithTlv(
            [[{
                type: 'CompressedOnly',
                data: {
                    delegatedAmount: 42n,
                    withheldTransferFee: 0n,
                    isFrozen: false,
                    compressionIndex: 1,
                    isAta: true,
                    bump: 200,
                    ownerIndex: 3,
                },
            }]],
            null,
        );
        const encoded = encodeTransfer2InstructionData(data);
        expect(encoded[encoded.length - 1]).toBe(0); // outTlv None

        // inTlv: Some(1) + outer(4) + inner(4) + disc(1) + CompressedOnly(21) = 31 before outTlv
        const inTlvStart = encoded.length - 1 - 31;
        expect(encoded[inTlvStart]).toBe(1); // Some
        const extStart = inTlvStart + 9; // after Some + outerLen + innerLen
        expect(encoded[extStart]).toBe(31); // CompressedOnly disc
        const view = new DataView(encoded.buffer, encoded.byteOffset);
        expect(view.getBigUint64(extStart + 1, true)).toBe(42n); // delegatedAmount
        expect(view.getBigUint64(extStart + 9, true)).toBe(0n); // withheldTransferFee
        expect(encoded[extStart + 17]).toBe(0); // isFrozen
        expect(encoded[extStart + 18]).toBe(1); // compressionIndex
        expect(encoded[extStart + 19]).toBe(1); // isAta
        expect(encoded[extStart + 20]).toBe(200); // bump
        expect(encoded[extStart + 21]).toBe(3); // ownerIndex
    });
});
