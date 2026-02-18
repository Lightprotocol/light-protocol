/**
 * Compressible extension codecs using Solana Kit patterns.
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
    getU32Decoder,
    getU32Encoder,
    getBytesDecoder,
    getBytesEncoder,
    getArrayDecoder,
    getArrayEncoder,
    addDecoderSizePrefix,
    addEncoderSizePrefix,
    getOptionEncoder,
    getOptionDecoder,
    fixEncoderSize,
    fixDecoderSize,
} from '@solana/codecs';

import type {
    CompressToPubkey,
    CompressibleExtensionInstructionData,
    CreateAtaInstructionData,
} from './types.js';

import { DISCRIMINATOR } from '../constants.js';

// ============================================================================
// VEC CODEC (Borsh-style: u32 length prefix)
// ============================================================================

function getVecEncoder<T>(itemEncoder: Encoder<T>): Encoder<T[]> {
    return addEncoderSizePrefix(
        getArrayEncoder(itemEncoder),
        getU32Encoder(),
    ) as Encoder<T[]>;
}

function getVecDecoder<T>(itemDecoder: Decoder<T>): Decoder<T[]> {
    return addDecoderSizePrefix(getArrayDecoder(itemDecoder), getU32Decoder());
}

// ============================================================================
// COMPRESS TO PUBKEY CODEC
// ============================================================================

export const getCompressToPubkeyEncoder = (): Encoder<CompressToPubkey> =>
    getStructEncoder([
        ['bump', getU8Encoder()],
        ['programId', fixEncoderSize(getBytesEncoder(), 32)],
        ['seeds', getVecEncoder(getVecEncoder(getU8Encoder()))],
    ]) as unknown as Encoder<CompressToPubkey>;

export const getCompressToPubkeyDecoder = (): Decoder<CompressToPubkey> =>
    getStructDecoder([
        ['bump', getU8Decoder()],
        ['programId', fixDecoderSize(getBytesDecoder(), 32)],
        ['seeds', getVecDecoder(getVecDecoder(getU8Decoder()))],
    ]) as unknown as Decoder<CompressToPubkey>;

export const getCompressToPubkeyCodec = (): Codec<CompressToPubkey> =>
    combineCodec(getCompressToPubkeyEncoder(), getCompressToPubkeyDecoder());

// ============================================================================
// COMPRESSIBLE EXTENSION INSTRUCTION DATA CODEC
// ============================================================================

export const getCompressibleExtensionDataEncoder =
    (): Encoder<CompressibleExtensionInstructionData> =>
        getStructEncoder([
            ['tokenAccountVersion', getU8Encoder()],
            ['rentPayment', getU8Encoder()],
            ['compressionOnly', getU8Encoder()],
            ['writeTopUp', getU32Encoder()],
            [
                'compressToPubkey',
                getOptionEncoder(getCompressToPubkeyEncoder()),
            ],
        ]);

export const getCompressibleExtensionDataDecoder =
    (): Decoder<CompressibleExtensionInstructionData> =>
        getStructDecoder([
            ['tokenAccountVersion', getU8Decoder()],
            ['rentPayment', getU8Decoder()],
            ['compressionOnly', getU8Decoder()],
            ['writeTopUp', getU32Decoder()],
            [
                'compressToPubkey',
                getOptionDecoder(getCompressToPubkeyDecoder()),
            ],
        ]) as unknown as Decoder<CompressibleExtensionInstructionData>;

export const getCompressibleExtensionDataCodec =
    (): Codec<CompressibleExtensionInstructionData> =>
        combineCodec(
            getCompressibleExtensionDataEncoder(),
            getCompressibleExtensionDataDecoder(),
        );

// ============================================================================
// CREATE ATA INSTRUCTION DATA CODEC
// ============================================================================

export const getCreateAtaDataEncoder = (): Encoder<CreateAtaInstructionData> =>
    getStructEncoder([
        ['bump', getU8Encoder()],
        [
            'compressibleConfig',
            getOptionEncoder(getCompressibleExtensionDataEncoder()),
        ],
    ]);

export const getCreateAtaDataDecoder = (): Decoder<CreateAtaInstructionData> =>
    getStructDecoder([
        ['bump', getU8Decoder()],
        [
            'compressibleConfig',
            getOptionDecoder(getCompressibleExtensionDataDecoder()),
        ],
    ]) as unknown as Decoder<CreateAtaInstructionData>;

export const getCreateAtaDataCodec = (): Codec<CreateAtaInstructionData> =>
    combineCodec(getCreateAtaDataEncoder(), getCreateAtaDataDecoder());

// ============================================================================
// FULL INSTRUCTION ENCODERS
// ============================================================================

/**
 * Encodes the CreateAssociatedTokenAccount instruction data.
 */
export function encodeCreateAtaInstructionData(
    data: CreateAtaInstructionData,
    idempotent = false,
): Uint8Array {
    const discriminator = idempotent
        ? DISCRIMINATOR.CREATE_ATA_IDEMPOTENT
        : DISCRIMINATOR.CREATE_ATA;

    const dataEncoder = getCreateAtaDataEncoder();
    const dataBytes = dataEncoder.encode(data);

    const result = new Uint8Array(1 + dataBytes.length);
    result[0] = discriminator;
    result.set(new Uint8Array(dataBytes), 1);

    return result;
}

/**
 * Default compressible extension params for rent-free ATAs.
 */
export function defaultCompressibleParams(): CompressibleExtensionInstructionData {
    return {
        tokenAccountVersion: 0,
        rentPayment: 0,
        compressionOnly: 0,
        writeTopUp: 0,
        compressToPubkey: null,
    };
}
