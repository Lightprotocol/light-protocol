/**
 * Unit tests for load functions.
 *
 * Tests for:
 * - loadTokenAccountsForTransfer
 * - loadAllTokenAccounts
 * - loadTokenAccount
 * - getOutputTreeInfo
 * - needsValidityProof
 */

import { describe, it, expect, vi } from 'vitest';
import { address } from '@solana/addresses';

import {
    loadTokenAccountsForTransfer,
    loadAllTokenAccounts,
    loadTokenAccount,
    getOutputTreeInfo,
    needsValidityProof,
    type LightIndexer,
} from '../../src/index.js';

import {
    IndexerError,
    IndexerErrorCode,
    TreeType,
    AccountState,
    type TreeInfo,
    type CompressedTokenAccount,
    type CompressedAccount,
} from '@lightprotocol/token-sdk';

// ============================================================================
// TEST HELPERS
// ============================================================================

const MOCK_OWNER = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const MOCK_MINT = address('So11111111111111111111111111111111111111112');

function createMockIndexer(overrides?: Partial<LightIndexer>): LightIndexer {
    return {
        getCompressedAccount: vi.fn(),
        getCompressedAccountByHash: vi.fn(),
        getCompressedTokenAccountsByOwner: vi.fn(),
        getMultipleCompressedAccounts: vi.fn(),
        getValidityProof: vi.fn(),
        ...overrides,
    };
}

function createMockTokenAccount(amount: bigint): CompressedTokenAccount {
    const mockTreeInfo: TreeInfo = {
        tree: address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
        queue: address('SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7'),
        treeType: TreeType.StateV2,
    };
    const mockAccount: CompressedAccount = {
        hash: new Uint8Array(32),
        address: null,
        owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
        lamports: 0n,
        data: null,
        leafIndex: 0,
        treeInfo: mockTreeInfo,
        proveByIndex: false,
        seq: null,
        slotCreated: 0n,
    };
    return {
        token: {
            mint: MOCK_MINT,
            owner: MOCK_OWNER,
            amount,
            delegate: null,
            state: AccountState.Initialized,
            tlv: null,
        },
        account: mockAccount,
    };
}

function createMockTreeInfo(
    treeType: TreeType,
    nextTree?: TreeInfo,
): TreeInfo {
    return {
        tree: address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
        queue: address('SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7'),
        treeType,
        nextTreeInfo: nextTree,
    };
}

// ============================================================================
// TESTS: loadTokenAccountsForTransfer
// ============================================================================

describe('loadTokenAccountsForTransfer', () => {
    it('returns inputs, proof, and totalAmount on success', async () => {
        const accounts = [
            createMockTokenAccount(500n),
            createMockTokenAccount(300n),
        ];

        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [],
            addresses: [],
        };

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await loadTokenAccountsForTransfer(
            indexer,
            MOCK_OWNER,
            600n,
        );

        expect(result.inputs).toHaveLength(2);
        expect(result.proof).toBe(mockProof);
        expect(result.totalAmount).toBe(800n);

        // Verify each input has merkleContext
        for (const input of result.inputs) {
            expect(input.merkleContext).toBeDefined();
            expect(input.merkleContext.tree).toBeDefined();
            expect(input.merkleContext.queue).toBeDefined();
        }
    });

    it('throws IndexerError with NotFound when no accounts exist', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        await expect(
            loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 100n),
        ).rejects.toThrow(IndexerError);

        try {
            await loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 100n);
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(IndexerErrorCode.NotFound);
        }
    });

    it('throws IndexerError with InsufficientBalance when balance is too low', async () => {
        const accounts = [createMockTokenAccount(50n)];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        await expect(
            loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 1000n),
        ).rejects.toThrow(IndexerError);

        try {
            await loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 1000n);
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InsufficientBalance,
            );
        }
    });
});

// ============================================================================
// TESTS: loadAllTokenAccounts
// ============================================================================

describe('loadAllTokenAccounts', () => {
    it('returns items from a single page with no cursor', async () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
        ];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        const result = await loadAllTokenAccounts(indexer, MOCK_OWNER);

        expect(result).toHaveLength(2);
        expect(result[0].token.amount).toBe(100n);
        expect(result[1].token.amount).toBe(200n);
    });

    it('paginates through multiple pages using cursor', async () => {
        const page1 = [createMockTokenAccount(100n)];
        const page2 = [createMockTokenAccount(200n)];

        const mockFn = vi
            .fn()
            .mockResolvedValueOnce({
                context: { slot: 100n },
                value: { items: page1, cursor: 'cursor-abc' },
            })
            .mockResolvedValueOnce({
                context: { slot: 101n },
                value: { items: page2, cursor: null },
            });

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: mockFn,
        });

        const result = await loadAllTokenAccounts(indexer, MOCK_OWNER);

        expect(result).toHaveLength(2);
        expect(result[0].token.amount).toBe(100n);
        expect(result[1].token.amount).toBe(200n);
        expect(mockFn).toHaveBeenCalledTimes(2);
    });

    it('throws after exceeding maximum page limit', async () => {
        // Always return a cursor to trigger infinite pagination
        const mockFn = vi.fn().mockResolvedValue({
            context: { slot: 100n },
            value: { items: [createMockTokenAccount(1n)], cursor: 'next' },
        });

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: mockFn,
        });

        await expect(
            loadAllTokenAccounts(indexer, MOCK_OWNER),
        ).rejects.toThrow('Pagination exceeded maximum of 100 pages');
    });
});

// ============================================================================
// TESTS: loadTokenAccount
// ============================================================================

describe('loadTokenAccount', () => {
    it('returns the first matching account', async () => {
        const account = createMockTokenAccount(500n);

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [account], cursor: null },
            }),
        });

        const result = await loadTokenAccount(indexer, MOCK_OWNER, MOCK_MINT);

        expect(result).not.toBeNull();
        expect(result!.token.amount).toBe(500n);
    });

    it('returns null when no accounts match', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        const result = await loadTokenAccount(indexer, MOCK_OWNER, MOCK_MINT);

        expect(result).toBeNull();
    });
});

// ============================================================================
// TESTS: getOutputTreeInfo
// ============================================================================

describe('getOutputTreeInfo', () => {
    it('returns nextTreeInfo when present', () => {
        const nextTree = createMockTreeInfo(TreeType.StateV2);
        nextTree.tree = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');

        const currentTree = createMockTreeInfo(TreeType.StateV2, nextTree);

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(nextTree);
        expect(result.tree).toBe(nextTree.tree);
    });

    it('returns the current tree when no next tree exists', () => {
        const currentTree = createMockTreeInfo(TreeType.StateV2);

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(currentTree);
    });
});

// ============================================================================
// TESTS: needsValidityProof
// ============================================================================

describe('needsValidityProof', () => {
    it('returns true when proveByIndex is false', () => {
        const account: CompressedAccount = {
            hash: new Uint8Array(32),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 0n,
            data: null,
            leafIndex: 0,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: false,
            seq: null,
            slotCreated: 0n,
        };

        expect(needsValidityProof(account)).toBe(true);
    });

    it('returns false when proveByIndex is true', () => {
        const account: CompressedAccount = {
            hash: new Uint8Array(32),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 0n,
            data: null,
            leafIndex: 0,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: true,
            seq: null,
            slotCreated: 0n,
        };

        expect(needsValidityProof(account)).toBe(false);
    });
});
