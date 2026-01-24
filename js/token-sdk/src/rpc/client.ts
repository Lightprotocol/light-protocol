/**
 * Light RPC Client - Placeholder
 *
 * This module will provide RPC client functionality for querying compressed
 * accounts from Photon indexer and requesting validity proofs from the prover.
 *
 * NOT YET IMPLEMENTED - requires prover integration.
 */

import type { Address } from '@solana/addresses';

// ============================================================================
// TYPES (Placeholder interfaces for SDK use)
// ============================================================================

/**
 * Parsed compressed token account.
 */
export interface ParsedTokenAccount {
    /** Account hash */
    hash: Uint8Array;
    /** Token mint */
    mint: Address;
    /** Token owner */
    owner: Address;
    /** Token amount */
    amount: bigint;
    /** Delegate (if any) */
    delegate: Address | null;
    /** Account state */
    state: number;
    /** Merkle tree address */
    merkleTree: Address;
    /** Leaf index */
    leafIndex: number;
}

/**
 * Validity proof for compressed account operations.
 */
export interface ValidityProof {
    /** Groth16 proof element A */
    a: Uint8Array;
    /** Groth16 proof element B */
    b: Uint8Array;
    /** Groth16 proof element C */
    c: Uint8Array;
    /** Root indices */
    rootIndices: number[];
}

/**
 * Light RPC client interface.
 */
export interface LightRpcClient {
    /** Get compressed token accounts by owner */
    getTokenAccountsByOwner(
        owner: Address,
        mint?: Address,
    ): Promise<ParsedTokenAccount[]>;
    /** Get validity proof for account hashes */
    getValidityProof(hashes: Uint8Array[]): Promise<ValidityProof>;
}

// ============================================================================
// FACTORY FUNCTION (Placeholder)
// ============================================================================

/**
 * Creates a Light RPC client.
 *
 * @param _endpoint - RPC endpoint (unused in placeholder)
 * @returns Never - throws an error
 * @throws Error indicating the client is not yet implemented
 *
 * @example
 * ```typescript
 * // Future usage:
 * const client = createLightRpcClient('https://photon.helius.dev');
 * const accounts = await client.getTokenAccountsByOwner(owner);
 * const proof = await client.getValidityProof(accounts.map(a => a.hash));
 * ```
 */
export function createLightRpcClient(_endpoint: string): LightRpcClient {
    throw new Error(
        'Light RPC client is not yet implemented. ' +
            'This feature requires Photon indexer and prover server integration.',
    );
}

/**
 * Checks if Light RPC services are available.
 *
 * @param _photonUrl - Photon indexer URL (unused in placeholder)
 * @param _proverUrl - Prover server URL (unused in placeholder)
 * @returns Always false in placeholder
 */
export async function isLightRpcAvailable(
    _photonUrl?: string,
    _proverUrl?: string,
): Promise<boolean> {
    return false;
}
