/**
 * Unit tests for Light Token Client
 *
 * Tests for:
 * - Account selection algorithm (selectAccountsForAmount)
 * - Tree info helpers (getOutputTreeInfo)
 * - Proof helpers (needsValidityProof)
 * - IndexerError
 * - V2-only tree validation (assertV2Tree)
 */

import { describe, it, expect } from 'vitest';
import { address, type Address } from '@solana/addresses';

import {
    selectAccountsForAmount,
    getOutputTreeInfo,
    needsValidityProof,
} from '../../src/index.js';

import {
    assertV2Tree,
    TreeType,
    IndexerError,
    IndexerErrorCode,
    type CompressedTokenAccount,
    type CompressedAccount,
    type TreeInfo,
} from '@lightprotocol/token-sdk';

// ============================================================================
// TEST HELPERS
// ============================================================================

// Valid test addresses (32-44 chars base58)
const MOCK_TREE = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
const MOCK_QUEUE = address('SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7');
const MOCK_OWNER = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const MOCK_MINT = address('So11111111111111111111111111111111111111112');
const MOCK_PROGRAM = address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m');

/**
 * Create a minimal mock CompressedTokenAccount for testing.
 */
function createMockTokenAccount(
    amount: bigint,
    owner: Address = MOCK_OWNER,
): CompressedTokenAccount {
    const mockTreeInfo: TreeInfo = {
        tree: MOCK_TREE,
        queue: MOCK_QUEUE,
        treeType: TreeType.StateV2,
    };

    const mockAccount: CompressedAccount = {
        hash: new Uint8Array(32),
        address: null,
        owner: MOCK_PROGRAM,
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
            owner,
            amount,
            delegate: null,
            state: 0,
            tlv: null,
        },
        account: mockAccount,
    };
}

/**
 * Create a mock TreeInfo for testing.
 */
function createMockTreeInfo(treeType: TreeType, nextTree?: TreeInfo): TreeInfo {
    return {
        tree: MOCK_TREE,
        queue: MOCK_QUEUE,
        treeType,
        nextTreeInfo: nextTree,
    };
}

// ============================================================================
// TEST: Account Selection Algorithm (selectAccountsForAmount)
// ============================================================================

describe('selectAccountsForAmount', () => {
    it('1.1 selects single account when it has enough balance', () => {
        const accounts = [
            createMockTokenAccount(1000n),
            createMockTokenAccount(500n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 800n);

        expect(result.accounts.length).toBe(1);
        expect(result.accounts[0].token.amount).toBe(1000n);
        expect(result.totalAmount).toBe(1000n);
    });

    it('1.2 selects multiple accounts to meet required amount (greedy, largest first)', () => {
        const accounts = [
            createMockTokenAccount(300n),
            createMockTokenAccount(500n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 700n);

        // Should select 500 first (largest), then 300
        expect(result.accounts.length).toBe(2);
        expect(result.accounts[0].token.amount).toBe(500n);
        expect(result.accounts[1].token.amount).toBe(300n);
        expect(result.totalAmount).toBe(800n);
    });

    it('1.3 returns all accounts when total balance is insufficient', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
            createMockTokenAccount(50n),
        ];

        const result = selectAccountsForAmount(accounts, 1000n);

        expect(result.accounts.length).toBe(3);
        expect(result.totalAmount).toBe(350n);
    });

    it('1.4 handles empty accounts array', () => {
        const result = selectAccountsForAmount([], 100n);

        expect(result.accounts.length).toBe(0);
        expect(result.totalAmount).toBe(0n);
    });

    it('1.5 handles zero required amount', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 0n);

        expect(result.accounts.length).toBe(0);
        expect(result.totalAmount).toBe(0n);
    });

    it('1.6 handles exact match', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
            createMockTokenAccount(300n),
        ];

        const result = selectAccountsForAmount(accounts, 300n);

        // Should select only the 300 account
        expect(result.accounts.length).toBe(1);
        expect(result.accounts[0].token.amount).toBe(300n);
        expect(result.totalAmount).toBe(300n);
    });
});

// ============================================================================
// TEST: Tree Info Helpers (getOutputTreeInfo)
// ============================================================================

describe('getOutputTreeInfo', () => {
    it('2.1 returns nextTreeInfo when present', () => {
        const nextTree = createMockTreeInfo(TreeType.StateV2);
        // Use a different valid address for the next tree
        nextTree.tree = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');

        const currentTree = createMockTreeInfo(TreeType.StateV2, nextTree);

        const result = getOutputTreeInfo(currentTree);

        expect(result.tree).toBe(nextTree.tree);
        expect(result).toBe(nextTree);
    });

    it('2.2 returns current tree when no next tree', () => {
        const currentTree = createMockTreeInfo(TreeType.StateV2);

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(currentTree);
        expect(result.tree).toBe(currentTree.tree);
    });

    it('2.3 handles null nextTreeInfo', () => {
        const currentTree = createMockTreeInfo(TreeType.StateV2);
        currentTree.nextTreeInfo = null;

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(currentTree);
    });
});

// ============================================================================
// TEST: Proof Helpers (needsValidityProof)
// ============================================================================

describe('needsValidityProof', () => {
    it('3.1 returns true when proveByIndex is false', () => {
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

    it('3.2 returns false when proveByIndex is true', () => {
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

// ============================================================================
// TEST: IndexerError
// ============================================================================

describe('IndexerError', () => {
    it('4.1 constructs error with correct code, message, and cause', () => {
        const cause = new Error('Original error');
        const error = new IndexerError(
            IndexerErrorCode.NetworkError,
            'Connection failed',
            cause,
        );

        expect(error.code).toBe(IndexerErrorCode.NetworkError);
        expect(error.message).toBe('Connection failed');
        expect(error.cause).toBe(cause);
        expect(error.name).toBe('IndexerError');
        expect(error instanceof Error).toBe(true);
    });

    it('4.2 works without cause', () => {
        const error = new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'Bad response',
        );

        expect(error.code).toBe(IndexerErrorCode.InvalidResponse);
        expect(error.message).toBe('Bad response');
        expect(error.cause).toBeUndefined();
    });

    it('4.3 supports all error codes', () => {
        const codes = [
            IndexerErrorCode.NetworkError,
            IndexerErrorCode.InvalidResponse,
            IndexerErrorCode.RpcError,
            IndexerErrorCode.NotFound,
        ];

        for (const code of codes) {
            const error = new IndexerError(code, `Error: ${code}`);
            expect(error.code).toBe(code);
        }
    });
});

// ============================================================================
// TEST: V2-Only Tree Validation (assertV2Tree)
// ============================================================================

describe('assertV2Tree', () => {
    it('5.1 throws for StateV1 tree type', () => {
        expect(() => assertV2Tree(TreeType.StateV1)).toThrow(IndexerError);
        expect(() => assertV2Tree(TreeType.StateV1)).toThrow(
            'V1 tree types are not supported',
        );
    });

    it('5.2 throws for AddressV1 tree type', () => {
        expect(() => assertV2Tree(TreeType.AddressV1)).toThrow(IndexerError);
        expect(() => assertV2Tree(TreeType.AddressV1)).toThrow(
            'V1 tree types are not supported',
        );
    });

    it('5.3 passes for StateV2 tree type', () => {
        expect(() => assertV2Tree(TreeType.StateV2)).not.toThrow();
    });

    it('5.4 passes for AddressV2 tree type', () => {
        expect(() => assertV2Tree(TreeType.AddressV2)).not.toThrow();
    });

    it('5.5 thrown error has correct error code', () => {
        try {
            assertV2Tree(TreeType.StateV1);
            expect.fail('Should have thrown');
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InvalidResponse,
            );
        }
    });
});
