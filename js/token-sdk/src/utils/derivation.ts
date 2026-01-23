/**
 * PDA derivation utilities for Light Token accounts.
 */

import {
    type Address,
    address,
    getAddressCodec,
    getProgramDerivedAddress,
} from '@solana/addresses';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    COMPRESSED_MINT_SEED,
    POOL_SEED,
} from '../constants.js';

// ============================================================================
// ASSOCIATED TOKEN ACCOUNT
// ============================================================================

/**
 * Derives the associated token account address for a given owner and mint.
 *
 * Seeds: [owner, LIGHT_TOKEN_PROGRAM_ID, mint]
 *
 * @param owner - The token account owner
 * @param mint - The token mint address
 * @returns The derived ATA address and bump
 */
export async function deriveAssociatedTokenAddress(
    owner: Address,
    mint: Address,
): Promise<{ address: Address; bump: number }> {
    const programIdBytes = getAddressCodec().encode(LIGHT_TOKEN_PROGRAM_ID);

    const [derivedAddress, bump] = await getProgramDerivedAddress({
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        seeds: [
            getAddressCodec().encode(owner),
            programIdBytes,
            getAddressCodec().encode(mint),
        ],
    });

    return { address: derivedAddress, bump };
}

/**
 * Derives the ATA address and verifies the provided bump matches.
 *
 * @param owner - The token account owner
 * @param mint - The token mint address
 * @param bump - The expected PDA bump seed
 * @returns The derived ATA address
 * @throws Error if the provided bump does not match the derived bump
 */
export async function getAssociatedTokenAddressWithBump(
    owner: Address,
    mint: Address,
    bump: number,
): Promise<Address> {
    const { address: derivedAddress, bump: derivedBump } =
        await deriveAssociatedTokenAddress(owner, mint);

    if (derivedBump !== bump) {
        throw new Error(`Bump mismatch: expected ${bump}, got ${derivedBump}`);
    }

    return derivedAddress;
}

// ============================================================================
// LIGHT MINT
// ============================================================================

/**
 * Derives the Light mint PDA address from a mint signer.
 *
 * Seeds: ["compressed_mint", mintSigner]
 *
 * @param mintSigner - The mint signer/authority pubkey
 * @returns The derived mint address and bump
 */
export async function deriveMintAddress(
    mintSigner: Address,
): Promise<{ address: Address; bump: number }> {
    const [derivedAddress, bump] = await getProgramDerivedAddress({
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        seeds: [
            new TextEncoder().encode(COMPRESSED_MINT_SEED),
            getAddressCodec().encode(mintSigner),
        ],
    });

    return { address: derivedAddress, bump };
}

// ============================================================================
// SPL INTERFACE POOL
// ============================================================================

/**
 * Derives the SPL interface pool PDA address.
 *
 * Seeds: ["pool", mint] or ["pool", mint, index]
 *
 * @param mint - The token mint address
 * @param index - Optional pool index (for multi-pool mints)
 * @returns The derived pool address and bump
 */
export async function derivePoolAddress(
    mint: Address,
    index?: number,
): Promise<{ address: Address; bump: number }> {
    const mintBytes = getAddressCodec().encode(mint);
    const seeds: Uint8Array[] = [
        new TextEncoder().encode(POOL_SEED),
        new Uint8Array(mintBytes),
    ];

    if (index !== undefined) {
        // Add index as u16 little-endian
        const indexBytes = new Uint8Array(2);
        new DataView(indexBytes.buffer).setUint16(0, index, true);
        seeds.push(indexBytes);
    }

    const [derivedAddress, bump] = await getProgramDerivedAddress({
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        seeds,
    });

    return { address: derivedAddress, bump };
}

// ============================================================================
// CPI AUTHORITY
// ============================================================================

/**
 * Derives the CPI authority PDA.
 *
 * @returns The CPI authority address
 */
export async function deriveCpiAuthority(): Promise<Address> {
    // CPI authority is a known constant, but we can derive it for verification
    return address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');
}
