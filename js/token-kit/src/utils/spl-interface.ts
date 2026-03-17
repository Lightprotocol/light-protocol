/**
 * SPL interface pool info for wrap/unwrap operations.
 */

import type { Address } from '@solana/addresses';

import { derivePoolAddress } from './derivation.js';

/**
 * Information about an initialized SPL interface pool PDA.
 */
export interface SplInterfaceInfo {
    /** Pool PDA address */
    poolAddress: Address;
    /** Token program (SPL Token or Token-2022) */
    tokenProgram: Address;
    /** Pool index (0-4) */
    poolIndex: number;
    /** PDA bump */
    bump: number;
    /** Whether the pool account is initialized */
    isInitialized: boolean;
}

/**
 * Minimal RPC interface for fetching account info.
 */
interface RpcLike {
    getAccountInfo(
        address: Address,
        config?: { encoding: string },
    ): Promise<{ value: { owner: Address; data: unknown } | null }>;
}

/**
 * Fetches SPL interface pool info for a mint.
 *
 * Derives all 5 possible pool PDAs (indices 0-4), queries each,
 * and returns the first initialized one.
 *
 * @param rpc - RPC client with getAccountInfo
 * @param mint - The token mint address
 * @param tokenProgram - The SPL token program that owns the pool accounts
 * @returns The first initialized SplInterfaceInfo
 * @throws If no initialized pool is found
 */
export async function getSplInterfaceInfo(
    rpc: RpcLike,
    mint: Address,
    tokenProgram: Address,
): Promise<SplInterfaceInfo> {
    // Derive all 5 pool PDAs
    const poolDerivations = await Promise.all(
        [0, 1, 2, 3, 4].map((index) => derivePoolAddress(mint, index)),
    );

    // Fetch all pool accounts
    const accountResults = await Promise.all(
        poolDerivations.map((derivation) =>
            rpc.getAccountInfo(derivation.address, { encoding: 'base64' }),
        ),
    );

    // Find the first initialized pool
    for (let i = 0; i < accountResults.length; i++) {
        const result = accountResults[i];
        if (result.value !== null) {
            return {
                poolAddress: poolDerivations[i].address,
                tokenProgram,
                poolIndex: i,
                bump: poolDerivations[i].bump,
                isInitialized: true,
            };
        }
    }

    throw new Error(
        `No initialized SPL interface pool found for mint ${mint}`,
    );
}

/**
 * Fetches all 5 SPL interface pool PDAs for a mint.
 *
 * Returns info for all 5 pool slots (indices 0-4), whether initialized or not.
 * Use this when you need visibility into all pool slots.
 *
 * @param rpc - RPC client with getAccountInfo
 * @param mint - The token mint address
 * @param tokenProgram - The SPL token program that owns the pool accounts
 * @returns Array of 5 SplInterfaceInfo entries
 */
export async function getSplInterfaceInfos(
    rpc: RpcLike,
    mint: Address,
    tokenProgram: Address,
): Promise<SplInterfaceInfo[]> {
    const poolDerivations = await Promise.all(
        [0, 1, 2, 3, 4].map((index) => derivePoolAddress(mint, index)),
    );

    const accountResults = await Promise.all(
        poolDerivations.map((derivation) =>
            rpc.getAccountInfo(derivation.address, { encoding: 'base64' }),
        ),
    );

    return poolDerivations.map((derivation, i) => ({
        poolAddress: derivation.address,
        tokenProgram,
        poolIndex: i,
        bump: derivation.bump,
        isInitialized: accountResults[i].value !== null,
    }));
}

/**
 * Selects an SPL interface pool for a compress or mint-to operation.
 *
 * Picks a random initialized pool from the available slots.
 *
 * @param infos - Array of pool infos (from getSplInterfaceInfos)
 * @returns A randomly selected initialized pool
 * @throws If no initialized pools exist
 */
export function selectSplInterfaceInfo(
    infos: SplInterfaceInfo[],
): SplInterfaceInfo {
    const initialized = infos.filter((info) => info.isInitialized);
    if (initialized.length === 0) {
        throw new Error('No initialized SPL interface pools available');
    }
    return initialized[Math.floor(Math.random() * initialized.length)];
}

/**
 * Selects SPL interface pools for decompression with sufficient balance.
 *
 * Returns all initialized pools. Consumers can further filter by balance
 * if needed (requires fetching token account data for each pool).
 *
 * @param infos - Array of pool infos (from getSplInterfaceInfos)
 * @returns Array of initialized pool infos
 */
export function selectSplInterfaceInfosForDecompression(
    infos: SplInterfaceInfo[],
): SplInterfaceInfo[] {
    return infos.filter((info) => info.isInitialized);
}

/**
 * Derives SPL interface info without fetching on-chain state.
 *
 * Useful when creating a pool in the same transaction (you know
 * it will be initialized by the time you need it).
 *
 * @param mint - The token mint address
 * @param tokenProgram - The SPL token program
 * @param poolIndex - Pool index (0-4, default 0)
 * @returns Pre-derived SplInterfaceInfo
 */
export async function deriveSplInterfaceInfo(
    mint: Address,
    tokenProgram: Address,
    poolIndex = 0,
): Promise<SplInterfaceInfo> {
    const { address: poolAddress, bump } = await derivePoolAddress(
        mint,
        poolIndex,
    );
    return {
        poolAddress,
        tokenProgram,
        poolIndex,
        bump,
        isInitialized: true,
    };
}
