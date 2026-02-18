/**
 * PDA derivation utilities for Light Token accounts.
 */

import {
    type Address,
    getAddressCodec,
    getProgramDerivedAddress,
} from '@solana/addresses';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    COMPRESSED_MINT_SEED,
    POOL_SEED,
    RESTRICTED_POOL_SEED,
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
 * Seed format:
 * - Regular index 0: ["pool", mint]
 * - Regular index 1-4: ["pool", mint, index]
 * - Restricted index 0: ["pool", mint, "restricted"]
 * - Restricted index 1-4: ["pool", mint, "restricted", index]
 *
 * Restricted pools are required for mints with extensions:
 * Pausable, PermanentDelegate, TransferFeeConfig, TransferHook,
 * DefaultAccountState, MintCloseAuthority.
 *
 * @param mint - The token mint address
 * @param index - Pool index (0-4, default 0)
 * @param restricted - Whether to use restricted derivation path
 * @returns The derived pool address and bump
 */
export async function derivePoolAddress(
    mint: Address,
    index = 0,
    restricted = false,
): Promise<{ address: Address; bump: number }> {
    const mintBytes = getAddressCodec().encode(mint);
    const seeds: Uint8Array[] = [
        new TextEncoder().encode(POOL_SEED),
        new Uint8Array(mintBytes),
    ];

    if (restricted) {
        seeds.push(new TextEncoder().encode(RESTRICTED_POOL_SEED));
    }

    if (index > 0) {
        // Index as single u8 byte (matches Rust: let index_bytes = [index])
        seeds.push(new Uint8Array([index]));
    }

    const [derivedAddress, bump] = await getProgramDerivedAddress({
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        seeds,
    });

    return { address: derivedAddress, bump };
}
