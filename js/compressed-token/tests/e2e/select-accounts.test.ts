import { describe, it, expect, beforeAll, beforeEach, assert } from 'vitest';

import BN from 'bn.js';
import { ParsedTokenAccount } from '@lightprotocol/stateless.js';

import {
    selectMinCompressedTokenAccountsForTransfer,
    selectMinCompressedTokenAccountsForTransferIdempotent,
    selectSmartCompressedTokenAccountsForTransfer,
    selectSmartCompressedTokenAccountsForTransferIdempotent,
} from '../../src';
import { ERROR_NO_ACCOUNTS_FOUND } from '../../src/utils/select-input-accounts';

describe('selectMinCompressedTokenAccountsForTransfer', () => {
    it('min: should select the largest account for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: new BN(100) },
                compressedAccount: { lamports: new BN(10) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN(100))).toBe(true);
        expect(totalLamports!.eq(new BN(10))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(175))).toBe(true);
    });

    it('min: throws if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        expect(() =>
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            ),
        ).toThrow(
            'Insufficient balance for transfer. Required: 75, available: 30.',
        );
    });

    it('min: should select multiple accounts if needed', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('min: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = new BN(75);

        expect(() =>
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('min: should ignore accounts with zero balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(0) },
                compressedAccount: { lamports: new BN(0) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(75))).toBe(true);
        expect(totalLamports!.eq(new BN(7))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(75))).toBe(true);
    });

    it('min: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: new BN('1000000000000000000') },
                compressedAccount: { lamports: new BN('100000000000000000') },
            },
            {
                parsed: { amount: new BN('500000000000000000') },
                compressedAccount: { lamports: new BN('50000000000000000') },
            },
            {
                parsed: { amount: new BN('250000000000000000') },
                compressedAccount: { lamports: new BN('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN('1000000000000000000'))).toBe(true);
        expect(totalLamports!.eq(new BN('100000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(new BN('1750000000000000000'))).toBe(true);
    });

    it('min: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('min: should handle max inputs less than accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(80))).toBe(true);
    });

    it('min: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(100);
        const maxInputs = 2;

        expect(() =>
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            ),
        ).toThrow(
            'Account limit exceeded: max 80 (2 accounts) per transaction. Total balance: 105 (3 accounts). Consider multiple transfers to spend full balance.',
        );
    });
});

describe('selectMinCompressedTokenAccountsForTransferIdempotent', () => {
    it('min idempotent: should select the largest account for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: new BN(100) },
                compressedAccount: { lamports: new BN(10) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN(100))).toBe(true);
        expect(totalLamports!.eq(new BN(10))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(175))).toBe(true);
    });

    it('min idempotent: should return the maximum possible amount if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN(30))).toBe(true);
        expect(totalLamports!.eq(new BN(3))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(30))).toBe(true);
    });

    it('min idempotent: should select multiple accounts if needed', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('min idempotent: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = new BN(75);

        expect(() =>
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('min idempotent: should ignore accounts with zero balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(0) },
                compressedAccount: { lamports: new BN(0) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(75))).toBe(true);
        expect(totalLamports!.eq(new BN(7))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(75))).toBe(true);
    });

    it('min idempotent: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: new BN('1000000000000000000') },
                compressedAccount: { lamports: new BN('100000000000000000') },
            },
            {
                parsed: { amount: new BN('500000000000000000') },
                compressedAccount: { lamports: new BN('50000000000000000') },
            },
            {
                parsed: { amount: new BN('250000000000000000') },
                compressedAccount: { lamports: new BN('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN('1000000000000000000'))).toBe(true);
        expect(totalLamports!.eq(new BN('100000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(new BN('1750000000000000000'))).toBe(true);
    });

    it('min idempotent: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
            {
                parsed: { amount: new BN(10) },
                compressedAccount: { lamports: new BN(1) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('min idempotent: should handle max inputs less than accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(80))).toBe(true);
    });

    it('min idempotent: should succeed and select 2 accounts with total 80', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(100);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(80))).toBe(true);
    });
});

describe('selectSmartCompressedTokenAccountsForTransfer', () => {
    it('smart: should select largest and smallest accounts for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: new BN(100) },
                compressedAccount: { lamports: new BN(10) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(125))).toBe(true);
        expect(totalLamports!.eq(new BN(12))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(175))).toBe(true);
    });

    it('smart: throws if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        expect(() =>
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            ),
        ).toThrow('Insufficient balance. Required: 75, available: 30.');
    });

    it('smart: should select 3 accounts if 2 are needed', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(new BN(105))).toBe(true);
        expect(totalLamports!.eq(new BN(10))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('smart: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = new BN(75);

        expect(() =>
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('smart: should ignore accounts with zero balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(0) },
                compressedAccount: { lamports: new BN(0) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(75))).toBe(true);
        expect(totalLamports!.eq(new BN(7))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(75))).toBe(true);
    });

    it('smart: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: new BN('1000000000000000000') },
                compressedAccount: { lamports: new BN('100000000000000000') },
            },
            {
                parsed: { amount: new BN('500000000000000000') },
                compressedAccount: { lamports: new BN('50000000000000000') },
            },
            {
                parsed: { amount: new BN('250000000000000000') },
                compressedAccount: { lamports: new BN('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN('1250000000000000000'))).toBe(true);
        expect(totalLamports!.eq(new BN('125000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(new BN('1750000000000000000'))).toBe(true);
    });

    it('smart: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(new BN(105))).toBe(true);
        expect(totalLamports!.eq(new BN(10))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('smart: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(100);
        const maxInputs = 2;

        expect(() =>
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            ),
        ).toThrow(
            'Account limit exceeded: max 80 (2 accounts) per transaction. Total balance: 105 (3 accounts). Consider multiple transfers to spend full balance.',
        );
    });

    it('smart: should handle max inputs less than accounts length', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(80))).toBe(true);
    });

    it('smart: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(100);
        const maxInputs = 2;

        expect(() =>
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            ),
        ).toThrow(
            'Account limit exceeded: max 80 (2 accounts) per transaction. Total balance: 105 (3 accounts). Consider multiple transfers to spend full balance.',
        );
    });
});

describe('selectSmartCompressedTokenAccountsForTransferIdempotent', () => {
    it('smart-idempotent: should select 2 accounts for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: new BN(100) },
                compressedAccount: { lamports: new BN(10) },
            },
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(125))).toBe(true);
        expect(totalLamports!.eq(new BN(12))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(175))).toBe(true);
    });

    it('smart-idempotent: should return the maximum possible amount if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(new BN(30))).toBe(true);
        expect(totalLamports!.eq(new BN(3))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(30))).toBe(true);
    });

    it('smart-idempotent: should select multiple accounts if needed', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(new BN(105))).toBe(true);
        expect(totalLamports!.eq(new BN(10))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(105))).toBe(true);
    });

    it('smart-idempotent: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = new BN(75);

        expect(() =>
            selectSmartCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('smart-idempotent: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: new BN(50) },
                compressedAccount: { lamports: new BN(5) },
            },
            {
                parsed: { amount: new BN(30) },
                compressedAccount: { lamports: new BN(3) },
            },
            {
                parsed: { amount: new BN(25) },
                compressedAccount: { lamports: new BN(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = new BN(100);
        const maxInputs = 2;
        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferIdempotent(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(new BN(80))).toBe(true);
        expect(totalLamports!.eq(new BN(8))).toBe(true);
        expect(maxPossibleAmount.eq(new BN(80))).toBe(true);
    });
});
