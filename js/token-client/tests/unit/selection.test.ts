/**
 * Unit tests for account selection algorithm (selectAccountsForAmount).
 *
 * Tests the greedy largest-first selection strategy used to pick
 * compressed token accounts for transfers.
 */

import { describe, it, expect } from 'vitest';
import { address } from '@solana/addresses';

import { selectAccountsForAmount, DEFAULT_MAX_INPUTS } from '../../src/index.js';

import {
    type CompressedTokenAccount,
    type CompressedAccount,
    type TreeInfo,
    TreeType,
    AccountState,
} from '@lightprotocol/token-sdk';

// ============================================================================
// TEST HELPERS
// ============================================================================

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
            mint: address('So11111111111111111111111111111111111111112'),
            owner: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
            amount,
            delegate: null,
            state: AccountState.Initialized,
            tlv: null,
        },
        account: mockAccount,
    };
}

// ============================================================================
// TESTS
// ============================================================================

describe('selectAccountsForAmount', () => {
    it('selects single large account when sufficient', () => {
        const accounts = [
            createMockTokenAccount(1000n),
            createMockTokenAccount(500n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 800n);

        expect(result.accounts).toHaveLength(1);
        expect(result.accounts[0].token.amount).toBe(1000n);
        expect(result.totalAmount).toBe(1000n);
    });

    it('selects multiple accounts using greedy largest-first strategy', () => {
        const accounts = [
            createMockTokenAccount(300n),
            createMockTokenAccount(500n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 700n);

        // Largest first: 500, then 300
        expect(result.accounts).toHaveLength(2);
        expect(result.accounts[0].token.amount).toBe(500n);
        expect(result.accounts[1].token.amount).toBe(300n);
        expect(result.totalAmount).toBe(800n);
    });

    it('returns all accounts when total balance is insufficient', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
            createMockTokenAccount(50n),
        ];

        const result = selectAccountsForAmount(accounts, 1000n);

        expect(result.accounts).toHaveLength(3);
        expect(result.totalAmount).toBe(350n);
    });

    it('returns zero accounts for empty input', () => {
        const result = selectAccountsForAmount([], 100n);

        expect(result.accounts).toHaveLength(0);
        expect(result.totalAmount).toBe(0n);
    });

    it('returns zero accounts for zero required amount', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
        ];

        const result = selectAccountsForAmount(accounts, 0n);

        expect(result.accounts).toHaveLength(0);
        expect(result.totalAmount).toBe(0n);
    });

    it('selects exact match with a single account', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
            createMockTokenAccount(300n),
        ];

        const result = selectAccountsForAmount(accounts, 300n);

        expect(result.accounts).toHaveLength(1);
        expect(result.accounts[0].token.amount).toBe(300n);
        expect(result.totalAmount).toBe(300n);
    });

    it('handles already-sorted input correctly', () => {
        // Descending order (already sorted by the algorithm's preference)
        const accounts = [
            createMockTokenAccount(500n),
            createMockTokenAccount(300n),
            createMockTokenAccount(100n),
        ];

        const result = selectAccountsForAmount(accounts, 400n);

        expect(result.accounts).toHaveLength(1);
        expect(result.accounts[0].token.amount).toBe(500n);
        expect(result.totalAmount).toBe(500n);
    });

    it('handles unsorted input correctly', () => {
        // Reverse order (ascending), algorithm should still pick largest first
        const accounts = [
            createMockTokenAccount(50n),
            createMockTokenAccount(150n),
            createMockTokenAccount(400n),
            createMockTokenAccount(100n),
        ];

        const result = selectAccountsForAmount(accounts, 500n);

        // 400 first, then 150
        expect(result.accounts).toHaveLength(2);
        expect(result.accounts[0].token.amount).toBe(400n);
        expect(result.accounts[1].token.amount).toBe(150n);
        expect(result.totalAmount).toBe(550n);
    });

    it('handles large amounts up to max u64', () => {
        const maxU64 = 18446744073709551615n;
        const halfMax = 9223372036854775808n;

        const accounts = [
            createMockTokenAccount(halfMax),
            createMockTokenAccount(halfMax),
        ];

        const result = selectAccountsForAmount(accounts, maxU64);

        expect(result.accounts).toHaveLength(2);
        expect(result.totalAmount).toBe(halfMax + halfMax);
    });

    it('skips zero-balance accounts naturally since they do not contribute', () => {
        const accounts = [
            createMockTokenAccount(0n),
            createMockTokenAccount(0n),
            createMockTokenAccount(500n),
            createMockTokenAccount(0n),
        ];

        const result = selectAccountsForAmount(accounts, 300n);

        // Algorithm sorts descending: 500, 0, 0, 0
        // Picks 500 first which satisfies 300, stops.
        expect(result.accounts).toHaveLength(1);
        expect(result.accounts[0].token.amount).toBe(500n);
        expect(result.totalAmount).toBe(500n);
    });

    it('DEFAULT_MAX_INPUTS is 4', () => {
        expect(DEFAULT_MAX_INPUTS).toBe(4);
    });

    it('respects maxInputs cap (default 4)', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
        ];

        // Without explicit maxInputs, defaults to 4
        const result = selectAccountsForAmount(accounts, 600n);

        // Should select at most 4 accounts even though 6 would be needed
        expect(result.accounts).toHaveLength(4);
        expect(result.totalAmount).toBe(400n);
    });

    it('respects custom maxInputs', () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
            createMockTokenAccount(100n),
        ];

        const result = selectAccountsForAmount(accounts, 400n, 2);
        expect(result.accounts).toHaveLength(2);
        expect(result.totalAmount).toBe(200n);
    });

    it('maxInputs=1 selects only the largest account', () => {
        const accounts = [
            createMockTokenAccount(50n),
            createMockTokenAccount(300n),
            createMockTokenAccount(100n),
        ];

        const result = selectAccountsForAmount(accounts, 400n, 1);
        expect(result.accounts).toHaveLength(1);
        expect(result.accounts[0].token.amount).toBe(300n);
        expect(result.totalAmount).toBe(300n);
    });

    it('zero-balance accounts are skipped and do not count toward maxInputs', () => {
        const accounts = [
            createMockTokenAccount(0n),
            createMockTokenAccount(0n),
            createMockTokenAccount(0n),
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
        ];

        // maxInputs=2, but zero accounts should not count
        const result = selectAccountsForAmount(accounts, 300n, 2);
        expect(result.accounts).toHaveLength(2);
        expect(result.accounts[0].token.amount).toBe(200n);
        expect(result.accounts[1].token.amount).toBe(100n);
        expect(result.totalAmount).toBe(300n);
    });

    it('all-zero accounts returns empty selection', () => {
        const accounts = [
            createMockTokenAccount(0n),
            createMockTokenAccount(0n),
            createMockTokenAccount(0n),
        ];

        const result = selectAccountsForAmount(accounts, 100n);
        expect(result.accounts).toHaveLength(0);
        expect(result.totalAmount).toBe(0n);
    });
});
