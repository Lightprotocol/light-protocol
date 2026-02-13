/**
 * Light Token Client Load Functions
 *
 * Functions for loading compressed account data for use in transactions.
 * Implements the AccountInterface pattern from sdk-libs/client.
 */

import type { Address } from '@solana/addresses';

import type { LightIndexer } from './indexer.js';
import {
    IndexerError,
    IndexerErrorCode,
    type CompressedAccount,
    type CompressedTokenAccount,
    type ValidityProofWithContext,
    type GetCompressedTokenAccountsOptions,
    type TreeInfo,
} from '@lightprotocol/token-sdk';

// ============================================================================
// ACCOUNT INTERFACE TYPES
// ============================================================================

/**
 * Input account for building transfer instructions.
 *
 * Contains the token account data and proof context needed for the transaction.
 */
export interface InputTokenAccount {
    /** The compressed token account */
    tokenAccount: CompressedTokenAccount;
    /** Merkle context for the account */
    merkleContext: MerkleContext;
}

/**
 * Merkle context for a compressed account.
 */
export interface MerkleContext {
    /** Merkle tree pubkey */
    tree: Address;
    /** Queue pubkey */
    queue: Address;
    /** Leaf index in the tree */
    leafIndex: number;
    /** Whether to prove by index */
    proveByIndex: boolean;
}

/**
 * Loaded token accounts with validity proof.
 *
 * This is the result of loading token accounts for a transaction.
 * Contains all the data needed to build transfer instructions.
 */
export interface LoadedTokenAccounts {
    /** Input token accounts with their merkle contexts */
    inputs: InputTokenAccount[];
    /** Validity proof for all inputs */
    proof: ValidityProofWithContext;
    /** Total amount available across all inputs */
    totalAmount: bigint;
}

/**
 * Options for loading token accounts.
 */
export interface LoadTokenAccountsOptions {
    /** Filter by mint */
    mint?: Address;
    /** Maximum number of accounts to load */
    limit?: number;
    /** Minimum amount required (will load accounts until this is met) */
    minAmount?: bigint;
}

// ============================================================================
// LOAD FUNCTIONS
// ============================================================================

/**
 * Load token accounts for a transfer.
 *
 * Fetches token accounts for the given owner, selects enough accounts
 * to meet the required amount, and fetches a validity proof.
 *
 * @param indexer - Light indexer client
 * @param owner - Token account owner
 * @param amount - Amount to transfer
 * @param options - Optional filters
 * @returns Loaded token accounts with proof
 * @throws Error if insufficient balance
 *
 * @example
 * ```typescript
 * const indexer = createLightIndexer('https://photon.helius.dev');
 * const loaded = await loadTokenAccountsForTransfer(
 *   indexer,
 *   owner,
 *   1000n,
 *   { mint: tokenMint }
 * );
 * // Use loaded.inputs and loaded.proof to build transfer instruction
 * ```
 */
export async function loadTokenAccountsForTransfer(
    indexer: LightIndexer,
    owner: Address,
    amount: bigint,
    options?: LoadTokenAccountsOptions,
): Promise<LoadedTokenAccounts> {
    // Fetch token accounts
    const fetchOptions: GetCompressedTokenAccountsOptions = {};
    if (options?.mint) {
        fetchOptions.mint = options.mint;
    }
    if (options?.limit) {
        fetchOptions.limit = options.limit;
    }

    const response = await indexer.getCompressedTokenAccountsByOwner(
        owner,
        fetchOptions,
    );

    const tokenAccounts = response.value.items;

    if (tokenAccounts.length === 0) {
        throw new IndexerError(
            IndexerErrorCode.NotFound,
            `No token accounts found for owner ${owner}`,
        );
    }

    // Select accounts to meet the required amount
    const selectedAccounts = selectAccountsForAmount(tokenAccounts, amount);

    if (selectedAccounts.totalAmount < amount) {
        throw new IndexerError(
            IndexerErrorCode.InsufficientBalance,
            `Insufficient balance: have ${selectedAccounts.totalAmount}, need ${amount}`,
        );
    }

    // Get validity proof for selected accounts
    const hashes = selectedAccounts.accounts.map((a) => a.account.hash);
    const proofResponse = await indexer.getValidityProof(hashes);

    // Build input accounts with merkle contexts
    const inputs: InputTokenAccount[] = selectedAccounts.accounts.map(
        (tokenAccount) => ({
            tokenAccount,
            merkleContext: {
                tree: tokenAccount.account.treeInfo.tree,
                queue: tokenAccount.account.treeInfo.queue,
                leafIndex: tokenAccount.account.leafIndex,
                proveByIndex: tokenAccount.account.proveByIndex,
            },
        }),
    );

    return {
        inputs,
        proof: proofResponse.value,
        totalAmount: selectedAccounts.totalAmount,
    };
}

/**
 * Load a single token account by owner and mint (ATA pattern).
 *
 * @param indexer - Light indexer client
 * @param owner - Token account owner
 * @param mint - Token mint
 * @returns The token account or null if not found
 */
export async function loadTokenAccount(
    indexer: LightIndexer,
    owner: Address,
    mint: Address,
): Promise<CompressedTokenAccount | null> {
    const response = await indexer.getCompressedTokenAccountsByOwner(owner, {
        mint,
        limit: 1,
    });

    return response.value.items[0] ?? null;
}

/**
 * Load all token accounts for an owner.
 *
 * @param indexer - Light indexer client
 * @param owner - Token account owner
 * @param options - Optional filters
 * @returns Array of token accounts
 */
/** Maximum number of pages to fetch to prevent infinite pagination loops. */
const MAX_PAGES = 100;

export async function loadAllTokenAccounts(
    indexer: LightIndexer,
    owner: Address,
    options?: GetCompressedTokenAccountsOptions,
): Promise<CompressedTokenAccount[]> {
    const allAccounts: CompressedTokenAccount[] = [];
    let cursor: string | undefined = options?.cursor;
    let pages = 0;

    do {
        if (++pages > MAX_PAGES) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Pagination exceeded maximum of ${MAX_PAGES} pages`,
            );
        }

        const response = await indexer.getCompressedTokenAccountsByOwner(
            owner,
            { ...options, cursor },
        );

        allAccounts.push(...response.value.items);
        cursor = response.value.cursor ?? undefined;
    } while (cursor);

    return allAccounts;
}

/**
 * Load a compressed account by address.
 *
 * @param indexer - Light indexer client
 * @param address - 32-byte account address
 * @returns The compressed account or null if not found
 */
export async function loadCompressedAccount(
    indexer: LightIndexer,
    address: Uint8Array,
): Promise<CompressedAccount | null> {
    const response = await indexer.getCompressedAccount(address);
    return response.value;
}

/**
 * Load a compressed account by hash.
 *
 * @param indexer - Light indexer client
 * @param hash - 32-byte account hash
 * @returns The compressed account or null if not found
 */
export async function loadCompressedAccountByHash(
    indexer: LightIndexer,
    hash: Uint8Array,
): Promise<CompressedAccount | null> {
    const response = await indexer.getCompressedAccountByHash(hash);
    return response.value;
}

// ============================================================================
// ACCOUNT SELECTION
// ============================================================================

/**
 * Result of account selection.
 */
export interface SelectedAccounts {
    /** Selected accounts */
    accounts: CompressedTokenAccount[];
    /** Total amount across selected accounts */
    totalAmount: bigint;
}

/**
 * Select token accounts to meet the required amount.
 *
 * Uses a greedy algorithm that prefers larger accounts first
 * to minimize the number of inputs.
 *
 * @param accounts - Available token accounts
 * @param requiredAmount - Amount needed
 * @returns Selected accounts and their total amount
 */
export function selectAccountsForAmount(
    accounts: CompressedTokenAccount[],
    requiredAmount: bigint,
): SelectedAccounts {
    // Sort by amount descending (prefer larger accounts)
    const sorted = [...accounts].sort((a, b) => {
        const diff = b.token.amount - a.token.amount;
        return diff > 0n ? 1 : diff < 0n ? -1 : 0;
    });

    const selected: CompressedTokenAccount[] = [];
    let total = 0n;

    for (const account of sorted) {
        if (total >= requiredAmount) {
            break;
        }
        selected.push(account);
        total += account.token.amount;
    }

    return {
        accounts: selected,
        totalAmount: total,
    };
}

// ============================================================================
// PROOF HELPERS
// ============================================================================

/**
 * Get a validity proof for multiple token accounts.
 *
 * @param indexer - Light indexer client
 * @param accounts - Token accounts to prove
 * @returns Validity proof with context
 */
export async function getValidityProofForAccounts(
    indexer: LightIndexer,
    accounts: CompressedTokenAccount[],
): Promise<ValidityProofWithContext> {
    const hashes = accounts.map((a) => a.account.hash);
    const response = await indexer.getValidityProof(hashes);
    return response.value;
}

/**
 * Check if an account needs a validity proof or can prove by index.
 *
 * @param account - The compressed account
 * @returns True if validity proof is needed
 */
export function needsValidityProof(account: CompressedAccount): boolean {
    return !account.proveByIndex;
}

/**
 * Extract tree info from a compressed account.
 *
 * @param account - The compressed account
 * @returns Tree info
 */
export function getTreeInfo(account: CompressedAccount): TreeInfo {
    return account.treeInfo;
}

/**
 * Get the output tree for new state.
 *
 * If the tree has a next tree (tree is full), use that.
 * Otherwise use the current tree.
 *
 * @param treeInfo - Current tree info
 * @returns Tree info for output state
 */
export function getOutputTreeInfo(treeInfo: TreeInfo): TreeInfo {
    return treeInfo.nextTreeInfo ?? treeInfo;
}
