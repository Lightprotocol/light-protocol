/**
 * Codecs for simple CToken instructions (transfer, burn, mint-to, approve, etc.).
 *
 * Each instruction follows the pattern: discriminator (u8) + fields.
 * Having codecs gives us decoders for free, enabling roundtrip tests.
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
    getU64Decoder,
    getU64Encoder,
} from '@solana/codecs';

// ============================================================================
// AMOUNT-ONLY INSTRUCTIONS (transfer, mint-to, burn, approve)
// ============================================================================

export interface AmountInstructionData {
    discriminator: number;
    amount: bigint;
}

export const getAmountInstructionEncoder =
    (): Encoder<AmountInstructionData> =>
        getStructEncoder([
            ['discriminator', getU8Encoder()],
            ['amount', getU64Encoder()],
        ]);

export const getAmountInstructionDecoder =
    (): Decoder<AmountInstructionData> =>
        getStructDecoder([
            ['discriminator', getU8Decoder()],
            ['amount', getU64Decoder()],
        ]);

export const getAmountInstructionCodec = (): Codec<AmountInstructionData> =>
    combineCodec(getAmountInstructionEncoder(), getAmountInstructionDecoder());

// ============================================================================
// CHECKED INSTRUCTIONS (transfer-checked, mint-to-checked, burn-checked)
// ============================================================================

export interface CheckedInstructionData {
    discriminator: number;
    amount: bigint;
    decimals: number;
}

export const getCheckedInstructionEncoder =
    (): Encoder<CheckedInstructionData> =>
        getStructEncoder([
            ['discriminator', getU8Encoder()],
            ['amount', getU64Encoder()],
            ['decimals', getU8Encoder()],
        ]);

export const getCheckedInstructionDecoder =
    (): Decoder<CheckedInstructionData> =>
        getStructDecoder([
            ['discriminator', getU8Decoder()],
            ['amount', getU64Decoder()],
            ['decimals', getU8Decoder()],
        ]);

export const getCheckedInstructionCodec =
    (): Codec<CheckedInstructionData> =>
        combineCodec(
            getCheckedInstructionEncoder(),
            getCheckedInstructionDecoder(),
        );

// ============================================================================
// DISCRIMINATOR-ONLY INSTRUCTIONS (revoke, freeze, thaw, close)
// ============================================================================

export interface DiscriminatorOnlyData {
    discriminator: number;
}

export const getDiscriminatorOnlyEncoder = (): Encoder<DiscriminatorOnlyData> =>
    getStructEncoder([['discriminator', getU8Encoder()]]);

export const getDiscriminatorOnlyDecoder = (): Decoder<DiscriminatorOnlyData> =>
    getStructDecoder([['discriminator', getU8Decoder()]]);

export const getDiscriminatorOnlyCodec = (): Codec<DiscriminatorOnlyData> =>
    combineCodec(getDiscriminatorOnlyEncoder(), getDiscriminatorOnlyDecoder());

// ============================================================================
// MAX TOP-UP ENCODING HELPER
// ============================================================================

/**
 * Encodes optional maxTopUp as a variable-length suffix.
 *
 * The on-chain program detects the format by instruction data length:
 * - 9 bytes (disc + u64 amount) = legacy format, no maxTopUp
 * - 11 bytes (disc + u64 amount + u16 maxTopUp) = extended format
 *
 * This matches the Rust program's length-based format detection.
 */
export function encodeMaxTopUp(maxTopUp: number | undefined): Uint8Array {
    if (maxTopUp === undefined) {
        return new Uint8Array(0);
    }
    return new Uint8Array(getU16Encoder().encode(maxTopUp));
}

/**
 * Attempts to decode a maxTopUp u16 from instruction data at the given offset.
 * Returns undefined if there are not enough bytes remaining.
 */
export function decodeMaxTopUp(
    data: Uint8Array,
    offset: number,
): number | undefined {
    if (data.length <= offset) {
        return undefined;
    }
    return getU16Decoder().read(data, offset)[0];
}
