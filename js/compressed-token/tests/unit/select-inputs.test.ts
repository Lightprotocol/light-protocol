import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import { bn, TreeType } from '@lightprotocol/stateless.js';
import {
    selectInputsForAmount,
    MAX_INPUT_ACCOUNTS,
} from '../../src/v3/actions/load-ata';

/**
 * Build a minimal ParsedTokenAccount mock with only the fields that
 * selectInputsForAmount accesses: `parsed.amount`.
 */
function mockAccount(amount: bigint): any {
    return {
        parsed: {
            mint: PublicKey.default,
            owner: PublicKey.default,
            amount: bn(amount.toString()),
            delegate: null,
            state: 1,
            tlv: null,
        },
        compressedAccount: {
            hash: new Uint8Array(32),
            treeInfo: {
                tree: PublicKey.default,
                queue: PublicKey.default,
                treeType: TreeType.StateV2,
            },
            leafIndex: 0,
            proveByIndex: false,
            owner: PublicKey.default,
            lamports: bn(0),
            address: null,
            data: null,
            readOnly: false,
        },
    };
}

function amounts(accounts: any[]): bigint[] {
    return accounts.map((a: any) => BigInt(a.parsed.amount.toString()));
}

describe('selectInputsForAmount', () => {
    it('returns [] for empty accounts', () => {
        expect(selectInputsForAmount([], BigInt(100))).toEqual([]);
    });

    it('returns [] when neededAmount is 0', () => {
        const accs = [mockAccount(500n)];
        expect(selectInputsForAmount(accs, BigInt(0))).toEqual([]);
    });

    it('returns [] when neededAmount is negative', () => {
        const accs = [mockAccount(500n)];
        expect(selectInputsForAmount(accs, BigInt(-1))).toEqual([]);
    });

    it('returns 1 account when only 1 exists and covers amount', () => {
        const accs = [mockAccount(1000n)];
        const result = selectInputsForAmount(accs, BigInt(500));
        expect(result.length).toBe(1);
        expect(amounts(result)).toEqual([1000n]);
    });

    it('pads to available count when fewer than MAX_INPUT_ACCOUNTS exist', () => {
        // 5 accounts, only 3 needed for amount -> pads to 5 (< 8)
        const accs = [
            mockAccount(100n),
            mockAccount(200n),
            mockAccount(300n),
            mockAccount(50n),
            mockAccount(150n),
        ];
        const result = selectInputsForAmount(accs, BigInt(400));
        // 300 + 200 = 500 >= 400, so 2 needed. Pad to min(8, 5) = 5.
        expect(result.length).toBe(5);
    });

    it('pads to MAX_INPUT_ACCOUNTS when 8+ accounts exist', () => {
        // 10 accounts, only 2 needed for amount -> pads to 8
        const accs = Array.from({ length: 10 }, (_, i) =>
            mockAccount(BigInt((i + 1) * 100)),
        );
        // Amounts: 100..1000. Need 500. 1000 alone covers it (1 needed).
        // Pad to min(8, 10) = 8.
        const result = selectInputsForAmount(accs, BigInt(500));
        expect(result.length).toBe(MAX_INPUT_ACCOUNTS);
    });

    it('returns exactly 8 accounts from 20 when amount is covered by 3', () => {
        const accs = Array.from({ length: 20 }, (_, i) =>
            mockAccount(BigInt((i + 1) * 10)),
        );
        // Amounts: 10..200. Need 500.
        // Sorted desc: 200, 190, 180, ... 200+190+180 = 570 >= 500 (3 needed).
        // Pad to min(8, 20) = 8.
        const result = selectInputsForAmount(accs, BigInt(500));
        expect(result.length).toBe(MAX_INPUT_ACCOUNTS);
    });

    it('returns >8 when amount requires more than 8 inputs', () => {
        // 20 accounts of 100 each. Need 1500 -> 15 inputs needed.
        const accs = Array.from({ length: 20 }, () => mockAccount(100n));
        const result = selectInputsForAmount(accs, BigInt(1500));
        // 15 needed, > 8, so no padding -> 15
        expect(result.length).toBe(15);
    });

    it('returns all when amount requires all inputs', () => {
        const accs = Array.from({ length: 12 }, () => mockAccount(100n));
        // Need 1200 = all
        const result = selectInputsForAmount(accs, BigInt(1200));
        expect(result.length).toBe(12);
    });

    it('sorts output by amount descending (largest first)', () => {
        const accs = [
            mockAccount(50n),
            mockAccount(500n),
            mockAccount(200n),
            mockAccount(100n),
            mockAccount(300n),
        ];
        const result = selectInputsForAmount(accs, BigInt(100));
        const resultAmounts = amounts(result);
        // Should be sorted descending
        for (let i = 0; i < resultAmounts.length - 1; i++) {
            expect(resultAmounts[i]).toBeGreaterThanOrEqual(
                resultAmounts[i + 1],
            );
        }
        expect(resultAmounts[0]).toBe(500n);
    });

    it('handles all same-size accounts', () => {
        const accs = Array.from({ length: 10 }, () => mockAccount(100n));
        // Need 300 -> 3 needed, pad to 8
        const result = selectInputsForAmount(accs, BigInt(300));
        expect(result.length).toBe(MAX_INPUT_ACCOUNTS);
        expect(amounts(result).reduce((s, a) => s + a, 0n)).toBe(800n);
    });

    it('does not mutate the input array', () => {
        const accs = [mockAccount(100n), mockAccount(300n), mockAccount(200n)];
        const originalOrder = amounts(accs);
        selectInputsForAmount(accs, BigInt(150));
        expect(amounts(accs)).toEqual(originalOrder);
    });

    it('returns exactly needed count when all inputs are required (no padding beyond 8)', () => {
        // 10 accounts, 100 each. Need 900 -> 9 inputs needed. > 8, no padding.
        const accs = Array.from({ length: 10 }, () => mockAccount(100n));
        const result = selectInputsForAmount(accs, BigInt(900));
        expect(result.length).toBe(9);
    });

    it('returns 8 when exactly 8 inputs are needed', () => {
        // 12 accounts, 100 each. Need 800 -> exactly 8 needed = MAX_INPUT_ACCOUNTS.
        const accs = Array.from({ length: 12 }, () => mockAccount(100n));
        const result = selectInputsForAmount(accs, BigInt(800));
        expect(result.length).toBe(MAX_INPUT_ACCOUNTS);
    });
});
