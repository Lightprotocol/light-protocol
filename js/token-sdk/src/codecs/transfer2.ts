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

import type {
    Compression,
    PackedMerkleContext,
    MultiInputTokenDataWithContext,
    MultiTokenTransferOutputData,
    CompressedCpiContext,
    CompressedProof,
    Transfer2InstructionData,
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
        ['cpiContextAccountIndex', getU8Encoder()],
    ]);

export const getCpiContextDecoder = (): Decoder<CompressedCpiContext> =>
    getStructDecoder([
        ['setContext', getBooleanDecoder()],
        ['firstSetContext', getBooleanDecoder()],
        ['cpiContextAccountIndex', getU8Decoder()],
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
 * For now, we support null (None) or empty arrays.
 * Full extension serialization would require additional codec implementations.
 */
function encodeTlv(tlv: unknown[][] | null): Uint8Array {
    if (tlv === null) {
        // Option::None
        return new Uint8Array([0]);
    }

    // Option::Some + Vec<Vec<...>>
    const chunks: Uint8Array[] = [];

    // Option::Some
    chunks.push(new Uint8Array([1]));

    // Outer vec length (u32)
    const outerLen = new Uint8Array(4);
    new DataView(outerLen.buffer).setUint32(0, tlv.length, true);
    chunks.push(outerLen);

    // For each inner vec
    for (const innerVec of tlv) {
        if (innerVec.length > 0) {
            throw new Error(
                'TLV extension serialization is not yet implemented',
            );
        }

        // Inner vec length (u32)
        const innerLen = new Uint8Array(4);
        new DataView(innerLen.buffer).setUint32(0, innerVec.length, true);
        chunks.push(innerLen);
    }

    // Concatenate all chunks
    const totalLen = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const result = new Uint8Array(totalLen);
    let offset = 0;
    for (const chunk of chunks) {
        result.set(chunk, offset);
        offset += chunk.length;
    }

    return result;
}
