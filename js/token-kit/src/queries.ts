/**
 * Query functions for unified account and mint views.
 *
 * These provide aggregated views across hot (on-chain), cold (compressed),
 * and SPL token account balances.
 */

import type { Address } from '@solana/addresses';

import type { LightIndexer } from './indexer.js';
import { loadAllTokenAccounts } from './load.js';
import type {
    GetCompressedTokenAccountsOptions,
    CompressedTokenAccount,
} from './client/index.js';

// ============================================================================
// TYPES
// ============================================================================

/**
 * Unified view of a token account across all sources.
 */
export interface AtaInterface {
    /** On-chain Light Token associated token account balance (hot) */
    hotBalance: bigint;
    /** Compressed token account balance (cold) */
    coldBalance: bigint;
    /** SPL token account balance */
    splBalance: bigint;
    /** Total balance across all sources */
    totalBalance: bigint;
    /** Source breakdown */
    sources: TokenAccountSource[];
    /** Number of compressed accounts */
    coldAccountCount: number;
    /** Compressed token accounts (cold) */
    coldAccounts: CompressedTokenAccount[];
}

/**
 * Individual token account source with balance.
 */
export interface TokenAccountSource {
    /** Source type */
    type: 'hot' | 'cold' | 'spl';
    /** Account address or identifier */
    address: Address;
    /** Balance from this source */
    balance: bigint;
}

/**
 * Unified view of a mint.
 */
export interface MintInterface {
    /** Mint address */
    mint: Address;
    /** Whether the mint exists on-chain */
    exists: boolean;
    /** Mint decimals (from on-chain data, 0 if not found) */
    decimals: number;
    /** Total supply (from on-chain data, 0n if not found) */
    supply: bigint;
    /** Whether the mint has a freeze authority */
    hasFreezeAuthority: boolean;
}

// ============================================================================
// QUERY FUNCTIONS
// ============================================================================

/**
 * Fetches a unified view of token balances for an owner and mint.
 *
 * Aggregates balances from:
 * - On-chain Light Token associated token account (hot)
 * - Compressed token accounts (cold)
 * - SPL associated token account (if exists)
 *
 * @param rpc - RPC client
 * @param indexer - Light indexer
 * @param owner - Account owner
 * @param mint - Token mint
 * @param hotAccount - On-chain Light Token ATA address (optional)
 * @param splAccount - SPL ATA address (optional)
 * @returns Unified account interface
 */
export async function getAtaInterface(
    rpc: QueryRpc,
    indexer: LightIndexer,
    owner: Address,
    mint: Address,
    hotAccount?: Address,
    splAccount?: Address,
): Promise<AtaInterface> {
    const sources: TokenAccountSource[] = [];
    let hotBalance = 0n;
    let coldBalance = 0n;
    let splBalance = 0n;

    // Fetch hot balance (on-chain Light Token ATA)
    if (hotAccount) {
        try {
            const info = await rpc.getAccountInfo(hotAccount, {
                encoding: 'base64',
            });
            if (info.value) {
                const data = info.value.data;
                if (data && typeof data === 'object' && Array.isArray(data)) {
                    const bytes = Uint8Array.from(
                        atob(data[0] as string),
                        (c) => c.charCodeAt(0),
                    );
                    if (bytes.length >= 72) {
                        const view = new DataView(
                            bytes.buffer,
                            bytes.byteOffset,
                            bytes.byteLength,
                        );
                        hotBalance = view.getBigUint64(64, true);
                    }
                }
                sources.push({
                    type: 'hot',
                    address: hotAccount,
                    balance: hotBalance,
                });
            }
        } catch {
            // Account may not exist
        }
    }

    // Fetch cold balance (compressed token accounts)
    const coldAccounts = await loadAllTokenAccounts(indexer, owner, {
        mint,
    } as GetCompressedTokenAccountsOptions);
    coldBalance = coldAccounts.reduce(
        (sum, acc) => sum + acc.token.amount,
        0n,
    );
    if (coldAccounts.length > 0) {
        sources.push({
            type: 'cold',
            address: owner,
            balance: coldBalance,
        });
    }

    // Fetch SPL balance
    if (splAccount) {
        try {
            const info = await rpc.getAccountInfo(splAccount, {
                encoding: 'base64',
            });
            if (info.value) {
                const data = info.value.data;
                if (data && typeof data === 'object' && Array.isArray(data)) {
                    const bytes = Uint8Array.from(
                        atob(data[0] as string),
                        (c) => c.charCodeAt(0),
                    );
                    if (bytes.length >= 72) {
                        const view = new DataView(
                            bytes.buffer,
                            bytes.byteOffset,
                            bytes.byteLength,
                        );
                        splBalance = view.getBigUint64(64, true);
                    }
                }
                sources.push({
                    type: 'spl',
                    address: splAccount,
                    balance: splBalance,
                });
            }
        } catch {
            // Account may not exist
        }
    }

    return {
        hotBalance,
        coldBalance,
        splBalance,
        totalBalance: hotBalance + coldBalance + splBalance,
        sources,
        coldAccountCount: coldAccounts.length,
        coldAccounts,
    };
}

/**
 * Minimal RPC interface for query operations.
 */
export interface QueryRpc {
    getAccountInfo(
        address: Address,
        config?: { encoding: string },
    ): Promise<{
        value: { owner: Address; data: unknown; lamports?: number } | null;
    }>;
}

/**
 * Fetches the decimals for an on-chain mint account.
 *
 * Reads byte 44 from the SPL mint layout.
 *
 * @param rpc - RPC client
 * @param mint - Mint address
 * @returns Mint decimals
 * @throws Error if the mint does not exist or data is too short
 */
export async function getMintDecimals(
    rpc: QueryRpc,
    mint: Address,
): Promise<number> {
    const info = await rpc.getAccountInfo(mint, { encoding: 'base64' });
    if (!info.value) {
        throw new Error(`Mint account not found: ${mint}`);
    }
    const data = info.value.data;
    if (!data || typeof data !== 'object' || !Array.isArray(data)) {
        throw new Error(`Invalid mint account data for ${mint}`);
    }
    const bytes = Uint8Array.from(
        atob(data[0] as string),
        (c) => c.charCodeAt(0),
    );
    if (bytes.length < 45) {
        throw new Error(`Mint data too short: ${bytes.length} bytes`);
    }
    return bytes[44];
}

/**
 * Fetches a unified view of a mint.
 *
 * Reads the on-chain mint account to extract decimals, supply,
 * and freeze authority status.
 *
 * @param rpc - RPC client
 * @param mint - Mint address
 * @returns Mint interface
 */
export async function getMintInterface(
    rpc: QueryRpc,
    mint: Address,
): Promise<MintInterface> {
    try {
        const info = await rpc.getAccountInfo(mint, { encoding: 'base64' });
        if (!info.value) {
            return {
                mint,
                exists: false,
                decimals: 0,
                supply: 0n,
                hasFreezeAuthority: false,
            };
        }

        const data = info.value.data;
        let bytes: Uint8Array;
        if (data && typeof data === 'object' && Array.isArray(data)) {
            bytes = Uint8Array.from(
                atob(data[0] as string),
                (c) => c.charCodeAt(0),
            );
        } else {
            return {
                mint,
                exists: true,
                decimals: 0,
                supply: 0n,
                hasFreezeAuthority: false,
            };
        }

        if (bytes.length < 82) {
            return {
                mint,
                exists: true,
                decimals: 0,
                supply: 0n,
                hasFreezeAuthority: false,
            };
        }

        // SPL Mint layout:
        // 0-3: mintAuthorityOption (u32)
        // 4-35: mintAuthority (32 bytes)
        // 36-43: supply (u64 LE)
        // 44: decimals (u8)
        // 45: isInitialized (bool)
        // 46-49: freezeAuthorityOption (u32)
        // 50-81: freezeAuthority (32 bytes)
        const view = new DataView(
            bytes.buffer,
            bytes.byteOffset,
            bytes.byteLength,
        );
        const supply = view.getBigUint64(36, true);
        const decimals = bytes[44];
        const freezeAuthorityOption = view.getUint32(46, true);

        return {
            mint,
            exists: true,
            decimals,
            supply,
            hasFreezeAuthority: freezeAuthorityOption === 1,
        };
    } catch {
        return {
            mint,
            exists: false,
            decimals: 0,
            supply: 0n,
            hasFreezeAuthority: false,
        };
    }
}
