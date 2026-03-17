/**
 * Shared helpers for instruction builders.
 */

import type { ReadonlyUint8Array } from '@solana/codecs';

import { encodeMaxTopUp } from '../codecs/instructions.js';

/**
 * Builds instruction data by concatenating base bytes with an optional maxTopUp suffix.
 */
export function buildInstructionDataWithMaxTopUp(
    baseBytes: ReadonlyUint8Array,
    maxTopUp?: number,
): Uint8Array {
    const maxTopUpBytes = encodeMaxTopUp(maxTopUp);
    const data = new Uint8Array(baseBytes.length + maxTopUpBytes.length);
    data.set(baseBytes, 0);
    if (maxTopUpBytes.length > 0) {
        data.set(maxTopUpBytes, baseBytes.length);
    }
    return data;
}
