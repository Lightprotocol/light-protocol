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
import { getAddressCodec, type Address } from '@solana/addresses';

import type {
    CompressToPubkey,
    CompressibleExtensionInstructionData,
    CreateAtaInstructionData,
    CreateTokenAccountInstructionData,
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

// Seeds are Vec<Vec<u8>> which we encode as Vec<bytes> using u32 length-prefixed bytes.
// This correctly maps ReadonlyUint8Array[] ↔ Borsh Vec<Vec<u8>>.
const getSeedEncoder = () =>
    addEncoderSizePrefix(getBytesEncoder(), getU32Encoder());
const getSeedDecoder = () =>
    addDecoderSizePrefix(getBytesDecoder(), getU32Decoder());

export const getCompressToPubkeyEncoder = (): Encoder<CompressToPubkey> =>
    getStructEncoder([
        ['bump', getU8Encoder()],
        ['programId', fixEncoderSize(getBytesEncoder(), 32)],
        ['seeds', getVecEncoder(getSeedEncoder())],
    ]);

export const getCompressToPubkeyDecoder = (): Decoder<CompressToPubkey> =>
    getStructDecoder([
        ['bump', getU8Decoder()],
        ['programId', fixDecoderSize(getBytesDecoder(), 32)],
        ['seeds', getVecDecoder(getSeedDecoder())],
    ]);

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

// Cast needed: getOptionDecoder returns Option<T> but interface uses T | null.
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
        [
            'compressibleConfig',
            getOptionEncoder(getCompressibleExtensionDataEncoder()),
        ],
    ]);

// Cast needed: getOptionDecoder returns Option<T> but interface uses T | null.
export const getCreateAtaDataDecoder = (): Decoder<CreateAtaInstructionData> =>
    getStructDecoder([
        [
            'compressibleConfig',
            getOptionDecoder(getCompressibleExtensionDataDecoder()),
        ],
    ]) as unknown as Decoder<CreateAtaInstructionData>;

export const getCreateAtaDataCodec = (): Codec<CreateAtaInstructionData> =>
    combineCodec(getCreateAtaDataEncoder(), getCreateAtaDataDecoder());

// ============================================================================
// CREATE TOKEN ACCOUNT INSTRUCTION DATA CODEC
// ============================================================================

const getOwnerEncoder = (): Encoder<Address> =>
    getAddressCodec() as unknown as Encoder<Address>;

const getOwnerDecoder = (): Decoder<Address> =>
    getAddressCodec() as unknown as Decoder<Address>;

export const getCreateTokenAccountDataEncoder =
    (): Encoder<CreateTokenAccountInstructionData> =>
        getStructEncoder([
            ['owner', getOwnerEncoder()],
            [
                'compressibleConfig',
                getOptionEncoder(getCompressibleExtensionDataEncoder()),
            ],
        ]);

// Cast needed: getOptionDecoder returns Option<T> but interface uses T | null.
export const getCreateTokenAccountDataDecoder =
    (): Decoder<CreateTokenAccountInstructionData> =>
        getStructDecoder([
            ['owner', getOwnerDecoder()],
            [
                'compressibleConfig',
                getOptionDecoder(getCompressibleExtensionDataDecoder()),
            ],
        ]) as unknown as Decoder<CreateTokenAccountInstructionData>;

export const getCreateTokenAccountDataCodec =
    (): Codec<CreateTokenAccountInstructionData> =>
        combineCodec(
            getCreateTokenAccountDataEncoder(),
            getCreateTokenAccountDataDecoder(),
        );

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
 * Encodes the CreateTokenAccount instruction data.
 *
 * When `splCompatibleOwnerOnlyData` is true, this emits the SPL-compatible
 * owner-only payload (`[owner:32]`) instead of the full Borsh struct.
 */
export function encodeCreateTokenAccountInstructionData(
    data: CreateTokenAccountInstructionData,
    splCompatibleOwnerOnlyData = false,
): Uint8Array {
    let payload: Uint8Array;
    if (splCompatibleOwnerOnlyData) {
        payload = new Uint8Array(getAddressCodec().encode(data.owner));
    } else {
        const dataEncoder = getCreateTokenAccountDataEncoder();
        payload = new Uint8Array(dataEncoder.encode(data));
    }

    const result = new Uint8Array(1 + payload.length);
    result[0] = DISCRIMINATOR.CREATE_TOKEN_ACCOUNT;
    result.set(payload, 1);
    return result;
}

/**
 * Default compressible extension params for rent-free ATAs.
 *
 * Matches the Rust SDK defaults:
 * - tokenAccountVersion: 3 (ShaFlat hashing)
 * - rentPayment: 16 (16 epochs, ~24 hours)
 * - compressionOnly: 1 (required for ATAs)
 * - writeTopUp: 766 (per-write top-up, ~2 epochs rent)
 * - compressToPubkey: null (required null for ATAs)
 */
export function defaultCompressibleParams(): CompressibleExtensionInstructionData {
    return {
        tokenAccountVersion: 3,
        rentPayment: 16,
        compressionOnly: 1,
        writeTopUp: 766,
        compressToPubkey: null,
    };
}
