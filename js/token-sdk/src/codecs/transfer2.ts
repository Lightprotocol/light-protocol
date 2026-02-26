/**
 * Transfer2 instruction codecs using Solana Kit patterns.
 */

import {
    type Codec,
    type Decoder,
    type Encoder,
    combineCodec,
    getStructDecoder,
    getStructEncoder,
    getU8Decoder,
    getU8Encoder,
    getU16Decoder,
    getU16Encoder,
    getU32Decoder,
    getU32Encoder,
    getU64Decoder,
    getU64Encoder,
    getBooleanDecoder,
    getBooleanEncoder,
    getArrayDecoder,
    getArrayEncoder,
    getBytesDecoder,
    getBytesEncoder,
    addDecoderSizePrefix,
    addEncoderSizePrefix,
    getOptionEncoder,
    getOptionDecoder,
    fixEncoderSize,
    fixDecoderSize,
} from '@solana/codecs';

import type { Address } from '@solana/addresses';
import { getAddressCodec } from '@solana/addresses';
import type { ReadonlyUint8Array } from '@solana/codecs';

import type {
    Compression,
    PackedMerkleContext,
    MultiInputTokenDataWithContext,
    MultiTokenTransferOutputData,
    CompressedCpiContext,
    CompressedProof,
    Transfer2InstructionData,
    ExtensionInstructionData,
    TokenMetadataExtension,
    CompressedOnlyExtension,
    CompressionInfo,
    RentConfig,
} from './types.js';

import { DISCRIMINATOR } from '../constants.js';

// ============================================================================
// COMPRESSION CODEC
// ============================================================================

export const getCompressionEncoder = (): Encoder<Compression> =>
    getStructEncoder([
        ['mode', getU8Encoder()],
        ['amount', getU64Encoder()],
        ['mint', getU8Encoder()],
        ['sourceOrRecipient', getU8Encoder()],
        ['authority', getU8Encoder()],
        ['poolAccountIndex', getU8Encoder()],
        ['poolIndex', getU8Encoder()],
        ['bump', getU8Encoder()],
        ['decimals', getU8Encoder()],
    ]);

export const getCompressionDecoder = (): Decoder<Compression> =>
    getStructDecoder([
        ['mode', getU8Decoder()],
        ['amount', getU64Decoder()],
        ['mint', getU8Decoder()],
        ['sourceOrRecipient', getU8Decoder()],
        ['authority', getU8Decoder()],
        ['poolAccountIndex', getU8Decoder()],
        ['poolIndex', getU8Decoder()],
        ['bump', getU8Decoder()],
        ['decimals', getU8Decoder()],
    ]);

export const getCompressionCodec = (): Codec<Compression> =>
    combineCodec(getCompressionEncoder(), getCompressionDecoder());

// ============================================================================
// PACKED MERKLE CONTEXT CODEC
// ============================================================================

export const getPackedMerkleContextEncoder = (): Encoder<PackedMerkleContext> =>
    getStructEncoder([
        ['merkleTreePubkeyIndex', getU8Encoder()],
        ['queuePubkeyIndex', getU8Encoder()],
        ['leafIndex', getU32Encoder()],
        ['proveByIndex', getBooleanEncoder()],
    ]);

export const getPackedMerkleContextDecoder = (): Decoder<PackedMerkleContext> =>
    getStructDecoder([
        ['merkleTreePubkeyIndex', getU8Decoder()],
        ['queuePubkeyIndex', getU8Decoder()],
        ['leafIndex', getU32Decoder()],
        ['proveByIndex', getBooleanDecoder()],
    ]);

export const getPackedMerkleContextCodec = (): Codec<PackedMerkleContext> =>
    combineCodec(
        getPackedMerkleContextEncoder(),
        getPackedMerkleContextDecoder(),
    );

// ============================================================================
// INPUT TOKEN DATA CODEC
// ============================================================================

export const getMultiInputTokenDataEncoder =
    (): Encoder<MultiInputTokenDataWithContext> =>
        getStructEncoder([
            ['owner', getU8Encoder()],
            ['amount', getU64Encoder()],
            ['hasDelegate', getBooleanEncoder()],
            ['delegate', getU8Encoder()],
            ['mint', getU8Encoder()],
            ['version', getU8Encoder()],
            ['merkleContext', getPackedMerkleContextEncoder()],
            ['rootIndex', getU16Encoder()],
        ]);

export const getMultiInputTokenDataDecoder =
    (): Decoder<MultiInputTokenDataWithContext> =>
        getStructDecoder([
            ['owner', getU8Decoder()],
            ['amount', getU64Decoder()],
            ['hasDelegate', getBooleanDecoder()],
            ['delegate', getU8Decoder()],
            ['mint', getU8Decoder()],
            ['version', getU8Decoder()],
            ['merkleContext', getPackedMerkleContextDecoder()],
            ['rootIndex', getU16Decoder()],
        ]);

export const getMultiInputTokenDataCodec =
    (): Codec<MultiInputTokenDataWithContext> =>
        combineCodec(
            getMultiInputTokenDataEncoder(),
            getMultiInputTokenDataDecoder(),
        );

// ============================================================================
// OUTPUT TOKEN DATA CODEC
// ============================================================================

export const getMultiTokenOutputDataEncoder =
    (): Encoder<MultiTokenTransferOutputData> =>
        getStructEncoder([
            ['owner', getU8Encoder()],
            ['amount', getU64Encoder()],
            ['hasDelegate', getBooleanEncoder()],
            ['delegate', getU8Encoder()],
            ['mint', getU8Encoder()],
            ['version', getU8Encoder()],
        ]);

export const getMultiTokenOutputDataDecoder =
    (): Decoder<MultiTokenTransferOutputData> =>
        getStructDecoder([
            ['owner', getU8Decoder()],
            ['amount', getU64Decoder()],
            ['hasDelegate', getBooleanDecoder()],
            ['delegate', getU8Decoder()],
            ['mint', getU8Decoder()],
            ['version', getU8Decoder()],
        ]);

export const getMultiTokenOutputDataCodec =
    (): Codec<MultiTokenTransferOutputData> =>
        combineCodec(
            getMultiTokenOutputDataEncoder(),
            getMultiTokenOutputDataDecoder(),
        );

// ============================================================================
// CPI CONTEXT CODEC
// ============================================================================

export const getCpiContextEncoder = (): Encoder<CompressedCpiContext> =>
    getStructEncoder([
        ['setContext', getBooleanEncoder()],
        ['firstSetContext', getBooleanEncoder()],
    ]);

export const getCpiContextDecoder = (): Decoder<CompressedCpiContext> =>
    getStructDecoder([
        ['setContext', getBooleanDecoder()],
        ['firstSetContext', getBooleanDecoder()],
    ]);

export const getCpiContextCodec = (): Codec<CompressedCpiContext> =>
    combineCodec(getCpiContextEncoder(), getCpiContextDecoder());

// ============================================================================
// PROOF CODEC
// ============================================================================

export const getCompressedProofEncoder = (): Encoder<CompressedProof> =>
    getStructEncoder([
        ['a', fixEncoderSize(getBytesEncoder(), 32)],
        ['b', fixEncoderSize(getBytesEncoder(), 64)],
        ['c', fixEncoderSize(getBytesEncoder(), 32)],
    ]);

export const getCompressedProofDecoder = (): Decoder<CompressedProof> =>
    getStructDecoder([
        ['a', fixDecoderSize(getBytesDecoder(), 32)],
        ['b', fixDecoderSize(getBytesDecoder(), 64)],
        ['c', fixDecoderSize(getBytesDecoder(), 32)],
    ]);

export const getCompressedProofCodec = (): Codec<CompressedProof> =>
    combineCodec(getCompressedProofEncoder(), getCompressedProofDecoder());

// ============================================================================
// VECTOR CODECS (with u32 length prefix for Borsh compatibility)
// ============================================================================

/**
 * Creates an encoder for a Vec type (Borsh style: u32 length prefix).
 */
function getVecEncoder<T>(itemEncoder: Encoder<T>): Encoder<T[]> {
    return addEncoderSizePrefix(
        getArrayEncoder(itemEncoder),
        getU32Encoder(),
    ) as Encoder<T[]>;
}

/**
 * Creates a decoder for a Vec type (Borsh style: u32 length prefix).
 */
function getVecDecoder<T>(itemDecoder: Decoder<T>): Decoder<T[]> {
    return addDecoderSizePrefix(getArrayDecoder(itemDecoder), getU32Decoder());
}

// ============================================================================
// TRANSFER2 INSTRUCTION DATA CODEC (Base fields only)
// Note: TLV fields require manual serialization due to complex nested structures
// ============================================================================

/**
 * Base Transfer2 instruction data (without TLV fields).
 */
export interface Transfer2BaseInstructionData {
    withTransactionHash: boolean;
    withLamportsChangeAccountMerkleTreeIndex: boolean;
    lamportsChangeAccountMerkleTreeIndex: number;
    lamportsChangeAccountOwnerIndex: number;
    outputQueue: number;
    maxTopUp: number;
    cpiContext: CompressedCpiContext | null;
    compressions: readonly Compression[] | null;
    proof: CompressedProof | null;
    inTokenData: readonly MultiInputTokenDataWithContext[];
    outTokenData: readonly MultiTokenTransferOutputData[];
    inLamports: readonly bigint[] | null;
    outLamports: readonly bigint[] | null;
}

// The encoder/decoder use `as unknown` casts because Kit's getOptionEncoder
// accepts OptionOrNullable<T> (broader than T | null) and getOptionDecoder
// returns Option<T> (narrower than T | null). The binary format is correct;
// the casts bridge the Rust Option<T> ↔ TypeScript T | null mismatch.
export const getTransfer2BaseEncoder =
    (): Encoder<Transfer2BaseInstructionData> =>
        getStructEncoder([
            ['withTransactionHash', getBooleanEncoder()],
            ['withLamportsChangeAccountMerkleTreeIndex', getBooleanEncoder()],
            ['lamportsChangeAccountMerkleTreeIndex', getU8Encoder()],
            ['lamportsChangeAccountOwnerIndex', getU8Encoder()],
            ['outputQueue', getU8Encoder()],
            ['maxTopUp', getU16Encoder()],
            ['cpiContext', getOptionEncoder(getCpiContextEncoder())],
            [
                'compressions',
                getOptionEncoder(getVecEncoder(getCompressionEncoder())),
            ],
            ['proof', getOptionEncoder(getCompressedProofEncoder())],
            ['inTokenData', getVecEncoder(getMultiInputTokenDataEncoder())],
            ['outTokenData', getVecEncoder(getMultiTokenOutputDataEncoder())],
            ['inLamports', getOptionEncoder(getVecEncoder(getU64Encoder()))],
            ['outLamports', getOptionEncoder(getVecEncoder(getU64Encoder()))],
        ]) as unknown as Encoder<Transfer2BaseInstructionData>;

export const getTransfer2BaseDecoder =
    (): Decoder<Transfer2BaseInstructionData> =>
        getStructDecoder([
            ['withTransactionHash', getBooleanDecoder()],
            ['withLamportsChangeAccountMerkleTreeIndex', getBooleanDecoder()],
            ['lamportsChangeAccountMerkleTreeIndex', getU8Decoder()],
            ['lamportsChangeAccountOwnerIndex', getU8Decoder()],
            ['outputQueue', getU8Decoder()],
            ['maxTopUp', getU16Decoder()],
            ['cpiContext', getOptionDecoder(getCpiContextDecoder())],
            [
                'compressions',
                getOptionDecoder(getVecDecoder(getCompressionDecoder())),
            ],
            ['proof', getOptionDecoder(getCompressedProofDecoder())],
            ['inTokenData', getVecDecoder(getMultiInputTokenDataDecoder())],
            ['outTokenData', getVecDecoder(getMultiTokenOutputDataDecoder())],
            ['inLamports', getOptionDecoder(getVecDecoder(getU64Decoder()))],
            ['outLamports', getOptionDecoder(getVecDecoder(getU64Decoder()))],
        ]) as unknown as Decoder<Transfer2BaseInstructionData>;

// ============================================================================
// TRANSFER2 FULL ENCODER (with discriminator and TLV fields)
// ============================================================================

/**
 * Encodes the full Transfer2 instruction data including discriminator and TLV.
 */
export function encodeTransfer2InstructionData(
    data: Transfer2InstructionData,
): Uint8Array {
    const baseEncoder = getTransfer2BaseEncoder();

    // Encode base data
    const baseData: Transfer2BaseInstructionData = {
        withTransactionHash: data.withTransactionHash,
        withLamportsChangeAccountMerkleTreeIndex:
            data.withLamportsChangeAccountMerkleTreeIndex,
        lamportsChangeAccountMerkleTreeIndex:
            data.lamportsChangeAccountMerkleTreeIndex,
        lamportsChangeAccountOwnerIndex: data.lamportsChangeAccountOwnerIndex,
        outputQueue: data.outputQueue,
        maxTopUp: data.maxTopUp,
        cpiContext: data.cpiContext,
        compressions: data.compressions,
        proof: data.proof,
        inTokenData: data.inTokenData,
        outTokenData: data.outTokenData,
        inLamports: data.inLamports,
        outLamports: data.outLamports,
    };

    const baseBytes = baseEncoder.encode(baseData);

    // Encode TLV fields (Option<Vec<Vec<ExtensionInstructionData>>>)
    const inTlvBytes = encodeTlv(data.inTlv);
    const outTlvBytes = encodeTlv(data.outTlv);

    // Combine: discriminator + base + inTlv + outTlv
    const result = new Uint8Array(
        1 + baseBytes.length + inTlvBytes.length + outTlvBytes.length,
    );
    result[0] = DISCRIMINATOR.TRANSFER2;
    result.set(baseBytes, 1);
    result.set(inTlvBytes, 1 + baseBytes.length);
    result.set(outTlvBytes, 1 + baseBytes.length + inTlvBytes.length);

    return result;
}

/**
 * Encodes TLV data as Option<Vec<Vec<ExtensionInstructionData>>>.
 *
 * Borsh format:
 * - None: [0x00]
 * - Some: [0x01] [outer_len: u32] [inner_vec_0] [inner_vec_1] ...
 *   where each inner_vec = [len: u32] [ext_0] [ext_1] ...
 *   and each ext = [discriminant: u8] [data...]
 *
 * Extension discriminants match Rust enum variant indices:
 * - 19: TokenMetadata
 * - 31: CompressedOnly
 * - 32: Compressible
 */
function encodeTlv(
    tlv: ExtensionInstructionData[][] | null,
): Uint8Array {
    if (tlv === null) {
        return new Uint8Array([0]);
    }

    const chunks: Uint8Array[] = [];

    // Option::Some
    chunks.push(new Uint8Array([1]));

    // Outer vec length (u32)
    chunks.push(writeU32(tlv.length));

    for (const innerVec of tlv) {
        // Inner vec length (u32)
        chunks.push(writeU32(innerVec.length));

        for (const ext of innerVec) {
            chunks.push(encodeExtensionInstructionData(ext));
        }
    }

    return concatBytes(chunks);
}

function writeU32(value: number): Uint8Array {
    const buf = new Uint8Array(4);
    new DataView(buf.buffer).setUint32(0, value, true);
    return buf;
}

function writeU16(value: number): Uint8Array {
    const buf = new Uint8Array(2);
    new DataView(buf.buffer).setUint16(0, value, true);
    return buf;
}

function writeU64(value: bigint): Uint8Array {
    const buf = new Uint8Array(8);
    new DataView(buf.buffer).setBigUint64(0, value, true);
    return buf;
}

function writeBool(value: boolean): Uint8Array {
    return new Uint8Array([value ? 1 : 0]);
}

/** Borsh Vec<u8> encoding: u32 length + bytes */
function writeVecBytes(bytes: ReadonlyUint8Array): Uint8Array {
    return concatBytes([writeU32(bytes.length), new Uint8Array(bytes)]);
}

/** Borsh Option encoding: 0x00 for None, 0x01 + data for Some */
function writeOption(
    value: unknown | null,
    encoder: (v: unknown) => Uint8Array,
): Uint8Array {
    if (value === null || value === undefined) {
        return new Uint8Array([0]);
    }
    return concatBytes([new Uint8Array([1]), encoder(value)]);
}

function concatBytes(arrays: Uint8Array[]): Uint8Array {
    const totalLen = arrays.reduce((sum, a) => sum + a.length, 0);
    const result = new Uint8Array(totalLen);
    let offset = 0;
    for (const arr of arrays) {
        result.set(arr, offset);
        offset += arr.length;
    }
    return result;
}

/**
 * Encodes a single ExtensionInstructionData with its Borsh enum discriminant.
 */
export function encodeExtensionInstructionData(
    ext: ExtensionInstructionData,
): Uint8Array {
    switch (ext.type) {
        case 'TokenMetadata':
            return concatBytes([
                new Uint8Array([19]), // discriminant
                encodeTokenMetadata(ext.data),
            ]);
        case 'PausableAccount':
            // Marker extension: discriminant only, zero data bytes
            return new Uint8Array([27]);
        case 'PermanentDelegateAccount':
            // Marker extension: discriminant only, zero data bytes
            return new Uint8Array([28]);
        case 'TransferFeeAccount':
            // Rust Placeholder29: unit variant, discriminant only (no data)
            return new Uint8Array([29]);
        case 'TransferHookAccount':
            // Rust Placeholder30: unit variant, discriminant only (no data)
            return new Uint8Array([30]);
        case 'CompressedOnly':
            return concatBytes([
                new Uint8Array([31]), // discriminant
                encodeCompressedOnly(ext.data),
            ]);
        case 'Compressible':
            return concatBytes([
                new Uint8Array([32]), // discriminant
                encodeCompressionInfo(ext.data),
            ]);
    }
}

function encodeTokenMetadata(data: TokenMetadataExtension): Uint8Array {
    const chunks: Uint8Array[] = [];

    // Option<Pubkey> - update_authority
    chunks.push(
        writeOption(data.updateAuthority, (v) =>
            new Uint8Array(getAddressCodec().encode(v as Address)),
        ),
    );

    // Vec<u8> fields
    chunks.push(writeVecBytes(data.name));
    chunks.push(writeVecBytes(data.symbol));
    chunks.push(writeVecBytes(data.uri));

    // Option<Vec<AdditionalMetadata>>
    chunks.push(
        writeOption(data.additionalMetadata, (v) => {
            const items = v as Array<{
                key: ReadonlyUint8Array;
                value: ReadonlyUint8Array;
            }>;
            const parts: Uint8Array[] = [writeU32(items.length)];
            for (const item of items) {
                parts.push(writeVecBytes(item.key));
                parts.push(writeVecBytes(item.value));
            }
            return concatBytes(parts);
        }),
    );

    return concatBytes(chunks);
}

function encodeCompressedOnly(data: CompressedOnlyExtension): Uint8Array {
    return concatBytes([
        writeU64(data.delegatedAmount),
        writeU64(data.withheldTransferFee),
        writeBool(data.isFrozen),
        new Uint8Array([data.compressionIndex]),
        writeBool(data.isAta),
        new Uint8Array([data.bump]),
        new Uint8Array([data.ownerIndex]),
    ]);
}

function encodeCompressionInfo(data: CompressionInfo): Uint8Array {
    return concatBytes([
        writeU16(data.configAccountVersion),
        new Uint8Array([data.compressToPubkey]),
        new Uint8Array([data.accountVersion]),
        writeU32(data.lamportsPerWrite),
        new Uint8Array(data.compressionAuthority),
        new Uint8Array(data.rentSponsor),
        writeU64(data.lastClaimedSlot),
        writeU32(data.rentExemptionPaid),
        writeU32(data.reserved),
        encodeRentConfig(data.rentConfig),
    ]);
}

function encodeRentConfig(data: RentConfig): Uint8Array {
    return concatBytes([
        writeU16(data.baseRent),
        writeU16(data.compressionCost),
        new Uint8Array([data.lamportsPerBytePerEpoch]),
        new Uint8Array([data.maxFundedEpochs]),
        writeU16(data.maxTopUp),
    ]);
}
