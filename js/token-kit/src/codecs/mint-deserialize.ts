/**
 * Lightweight compressed mint account deserializer.
 *
 * Parses raw bytes from a compressed mint account using DataView.
 * No external dependencies needed — follows the same manual Borsh
 * pattern as queries.ts getMintInterface.
 */

import { EXTENSION_DISCRIMINANT } from '../constants.js';

// ============================================================================
// TYPES
// ============================================================================

/** Base SPL mint fields (82 bytes). */
export interface BaseMint {
    mintAuthorityOption: number;
    mintAuthority: Uint8Array;
    supply: bigint;
    decimals: number;
    isInitialized: boolean;
    freezeAuthorityOption: number;
    freezeAuthority: Uint8Array;
}

/** Light Protocol-specific mint context following the base mint. */
export interface DeserializedMintContext {
    version: number;
    cmintDecompressed: boolean;
    splMint: Uint8Array;
    mintSigner: Uint8Array;
    bump: number;
}

/** Full deserialized compressed mint. */
export interface DeserializedCompressedMint {
    base: BaseMint;
    mintContext: DeserializedMintContext;
    /** Index of the TokenMetadata extension in extensions array, or -1. */
    metadataExtensionIndex: number;
}

// ============================================================================
// DESERIALIZER
// ============================================================================

/**
 * Deserializes a compressed mint account from raw bytes.
 *
 * Layout:
 *   BaseMint (82 bytes):
 *     0-3:   mintAuthorityOption (u32 LE)
 *     4-35:  mintAuthority (32 bytes)
 *     36-43: supply (u64 LE)
 *     44:    decimals (u8)
 *     45:    isInitialized (bool)
 *     46-49: freezeAuthorityOption (u32 LE)
 *     50-81: freezeAuthority (32 bytes)
 *
 *   MintContext (67 bytes):
 *     82:     version (u8)
 *     83:     cmintDecompressed (bool)
 *     84-115: splMint (32 bytes)
 *     116-147: mintSigner (32 bytes)
 *     148:    bump (u8)
 *
 *   Extensions (variable, starting at offset 149):
 *     Scanned for TokenMetadata (discriminant 19).
 *
 * @param data - Raw account data bytes
 * @returns Deserialized compressed mint
 * @throws Error if data is too short
 */
export function deserializeCompressedMint(
    data: Uint8Array,
): DeserializedCompressedMint {
    if (data.length < 149) {
        throw new Error(
            `Compressed mint data too short: ${data.length} bytes, need at least 149`,
        );
    }

    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

    // BaseMint (82 bytes)
    const base: BaseMint = {
        mintAuthorityOption: view.getUint32(0, true),
        mintAuthority: data.slice(4, 36),
        supply: view.getBigUint64(36, true),
        decimals: data[44],
        isInitialized: data[45] !== 0,
        freezeAuthorityOption: view.getUint32(46, true),
        freezeAuthority: data.slice(50, 82),
    };

    // MintContext (67 bytes starting at offset 82)
    const mintContext: DeserializedMintContext = {
        version: data[82],
        cmintDecompressed: data[83] !== 0,
        splMint: data.slice(84, 116),
        mintSigner: data.slice(116, 148),
        bump: data[148],
    };

    // Scan extensions for TokenMetadata (discriminant 19)
    let metadataExtensionIndex = -1;
    if (data.length > 149) {
        // Extensions are a Vec: first 4 bytes = length (u32 LE)
        const extOffset = 149;
        if (data.length >= extOffset + 4) {
            const extCount = view.getUint32(extOffset, true);
            let pos = extOffset + 4;
            for (let i = 0; i < extCount && pos < data.length; i++) {
                // Each extension starts with a discriminant (u16 LE)
                if (pos + 2 > data.length) break;
                const disc = view.getUint16(pos, true);
                if (disc === EXTENSION_DISCRIMINANT.TOKEN_METADATA) {
                    metadataExtensionIndex = i;
                    break;
                }
                // Skip this extension — we don't know the exact size of every
                // variant, so we stop scanning after finding metadata or not.
                // For builders that need extensionIndex, -1 means "not found".
                break;
            }
        }
    }

    return { base, mintContext, metadataExtensionIndex };
}
