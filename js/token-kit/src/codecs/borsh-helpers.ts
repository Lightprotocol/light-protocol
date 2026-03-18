/**
 * Shared manual Borsh encoding helpers.
 *
 * Used by transfer2.ts and mint-action.ts for complex nested structures
 * that are too dynamic for Solana Kit's static struct codecs.
 */

import type { ReadonlyUint8Array, Encoder, Decoder } from '@solana/codecs';
import { getArrayEncoder, getArrayDecoder } from '@solana/codecs';

export function writeU8(value: number): Uint8Array {
    return new Uint8Array([value & 0xff]);
}

export function writeU16(value: number): Uint8Array {
    const buf = new Uint8Array(2);
    new DataView(buf.buffer).setUint16(0, value, true);
    return buf;
}

export function writeU32(value: number): Uint8Array {
    const buf = new Uint8Array(4);
    new DataView(buf.buffer).setUint32(0, value, true);
    return buf;
}

export function writeU64(value: bigint): Uint8Array {
    const buf = new Uint8Array(8);
    new DataView(buf.buffer).setBigUint64(0, value, true);
    return buf;
}

export function writeBool(value: boolean): Uint8Array {
    return new Uint8Array([value ? 1 : 0]);
}

/** Borsh Vec<u8> encoding: u32 length + bytes. */
export function writeVecBytes(bytes: ReadonlyUint8Array): Uint8Array {
    return concatBytes([writeU32(bytes.length), new Uint8Array(bytes)]);
}

/** Borsh Option encoding: 0x00 for None, 0x01 + data for Some. */
export function writeOption<T>(
    value: T | null | undefined,
    encoder: (v: T) => Uint8Array,
): Uint8Array {
    if (value === null || value === undefined) {
        return new Uint8Array([0]);
    }
    return concatBytes([new Uint8Array([1]), encoder(value)]);
}

export function concatBytes(arrays: Uint8Array[]): Uint8Array {
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
 * Creates an encoder for a Vec type (Borsh style: u32 element count prefix).
 *
 * Note: getArrayEncoder defaults to { size: getU32Encoder() } which is the
 * Borsh Vec format (u32 count + items). Do NOT wrap with addEncoderSizePrefix
 * which would add a byte-count prefix on top.
 */
export function getVecEncoder<T>(itemEncoder: Encoder<T>): Encoder<T[]> {
    return getArrayEncoder(itemEncoder) as Encoder<T[]>;
}

/**
 * Creates a decoder for a Vec type (Borsh style: u32 element count prefix).
 */
export function getVecDecoder<T>(itemDecoder: Decoder<T>): Decoder<T[]> {
    return getArrayDecoder(itemDecoder) as Decoder<T[]>;
}
