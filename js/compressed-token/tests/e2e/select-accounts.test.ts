import { bn } from '@lightprotocol/stateless.js';
import { describe, it, expect } from 'vitest';
import { ParsedTokenAccount } from '@lightprotocol/stateless.js';

import {
    selectMinCompressedTokenAccountsForTransfer,
    selectMinCompressedTokenAccountsForTransferOrPartial,
    selectSmartCompressedTokenAccountsForTransfer,
    selectSmartCompressedTokenAccountsForTransferOrPartial,
} from '../../src';
import { ERROR_NO_ACCOUNTS_FOUND } from '../../src/utils/select-input-accounts';

describe('selectMinCompressedTokenAccountsForTransfer', () => {
    it('min: should select the largest account for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: bn(100) },
                compressedAccount: { lamports: bn(10) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn(100))).toBe(true);
        expect(totalLamports!.eq(bn(10))).toBe(true);
        expect(maxPossibleAmount.eq(bn(175))).toBe(true);
    });

    it('min: throws if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

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
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('min: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = bn(75);

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
                parsed: { amount: bn(0) },
                compressedAccount: { lamports: bn(0) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(75))).toBe(true);
        expect(totalLamports!.eq(bn(7))).toBe(true);
        expect(maxPossibleAmount.eq(bn(75))).toBe(true);
    });

    it('min: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: bn('1000000000000000000') },
                compressedAccount: { lamports: bn('100000000000000000') },
            },
            {
                parsed: { amount: bn('500000000000000000') },
                compressedAccount: { lamports: bn('50000000000000000') },
            },
            {
                parsed: { amount: bn('250000000000000000') },
                compressedAccount: { lamports: bn('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn('1000000000000000000'))).toBe(true);
        expect(totalLamports!.eq(bn('100000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(bn('1750000000000000000'))).toBe(true);
    });

    it('min: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('min: should handle max inputs less than accounts length', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(80))).toBe(true);
    });

    it('min: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(100);
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

describe('selectMinCompressedTokenAccountsForTransferorPartial', () => {
    it('min orPartial: should select the largest account for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: bn(100) },
                compressedAccount: { lamports: bn(10) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn(100))).toBe(true);
        expect(totalLamports!.eq(bn(10))).toBe(true);
        expect(maxPossibleAmount.eq(bn(175))).toBe(true);
    });

    it('min orPartial: should return the maximum possible amount if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn(30))).toBe(true);
        expect(totalLamports!.eq(bn(3))).toBe(true);
        expect(maxPossibleAmount.eq(bn(30))).toBe(true);
    });

    it('min orPartial: should select multiple accounts if needed', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('min orPartial: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = bn(75);

        expect(() =>
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('min orPartial: should ignore accounts with zero balance', () => {
        const accounts = [
            {
                parsed: { amount: bn(0) },
                compressedAccount: { lamports: bn(0) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(75))).toBe(true);
        expect(totalLamports!.eq(bn(7))).toBe(true);
        expect(maxPossibleAmount.eq(bn(75))).toBe(true);
    });

    it('min orPartial: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: bn('1000000000000000000') },
                compressedAccount: { lamports: bn('100000000000000000') },
            },
            {
                parsed: { amount: bn('500000000000000000') },
                compressedAccount: { lamports: bn('50000000000000000') },
            },
            {
                parsed: { amount: bn('250000000000000000') },
                compressedAccount: { lamports: bn('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn('1000000000000000000'))).toBe(true);
        expect(totalLamports!.eq(bn('100000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(bn('1750000000000000000'))).toBe(true);
    });

    it('min orPartial: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
            {
                parsed: { amount: bn(10) },
                compressedAccount: { lamports: bn(1) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('min orPartial: should handle max inputs less than accounts length', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(80))).toBe(true);
    });

    it('min orPartial: should succeed and select 2 accounts with total 80', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(100);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectMinCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(80))).toBe(true);
    });
});

describe('selectSmartCompressedTokenAccountsForTransfer', () => {
    it('smart: should select largest and smallest accounts for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: bn(100) },
                compressedAccount: { lamports: bn(10) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(125))).toBe(true);
        expect(totalLamports!.eq(bn(12))).toBe(true);
        expect(maxPossibleAmount.eq(bn(175))).toBe(true);
    });

    it('smart: throws if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

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
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(bn(105))).toBe(true);
        expect(totalLamports!.eq(bn(10))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('smart: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = bn(75);

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
                parsed: { amount: bn(0) },
                compressedAccount: { lamports: bn(0) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(75))).toBe(true);
        expect(totalLamports!.eq(bn(7))).toBe(true);
        expect(maxPossibleAmount.eq(bn(75))).toBe(true);
    });

    it('smart: should handle large numbers', () => {
        const accounts = [
            {
                parsed: { amount: bn('1000000000000000000') },
                compressedAccount: { lamports: bn('100000000000000000') },
            },
            {
                parsed: { amount: bn('500000000000000000') },
                compressedAccount: { lamports: bn('50000000000000000') },
            },
            {
                parsed: { amount: bn('250000000000000000') },
                compressedAccount: { lamports: bn('25000000000000000') },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn('750000000000000000');

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn('1250000000000000000'))).toBe(true);
        expect(totalLamports!.eq(bn('125000000000000000'))).toBe(true);
        expect(maxPossibleAmount.eq(bn('1750000000000000000'))).toBe(true);
    });

    it('smart: should handle max inputs equal to accounts length', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 3;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(bn(105))).toBe(true);
        expect(totalLamports!.eq(bn(10))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('smart: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(100);
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
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);
        const maxInputs = 2;

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransfer(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(80))).toBe(true);
    });

    it('smart: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(100);
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

describe('selectSmartCompressedTokenAccountsForTransferOrPartial', () => {
    it('smart-orPartial: should select 2 accounts for a valid transfer where 1 account is enough', () => {
        const accounts = [
            {
                parsed: { amount: bn(100) },
                compressedAccount: { lamports: bn(10) },
            },
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(125))).toBe(true);
        expect(totalLamports!.eq(bn(12))).toBe(true);
        expect(maxPossibleAmount.eq(bn(175))).toBe(true);
    });

    it('smart-orPartial: should return the maximum possible amount if there is not enough balance', () => {
        const accounts = [
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(1);
        expect(total.eq(bn(30))).toBe(true);
        expect(totalLamports!.eq(bn(3))).toBe(true);
        expect(maxPossibleAmount.eq(bn(30))).toBe(true);
    });

    it('smart-orPartial: should select multiple accounts if needed', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(75);

        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            );

        expect(selectedAccounts.length).toBe(3);
        expect(total.eq(bn(105))).toBe(true);
        expect(totalLamports!.eq(bn(10))).toBe(true);
        expect(maxPossibleAmount.eq(bn(105))).toBe(true);
    });

    it('smart-orPartial: should handle empty accounts array', () => {
        const accounts: ParsedTokenAccount[] = [];
        const transferAmount = bn(75);

        expect(() =>
            selectSmartCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
            ),
        ).toThrow(ERROR_NO_ACCOUNTS_FOUND);
    });

    it('smart-orPartial: should throw if not enough accounts selected because of maxInputs lower than what WOULD be available', () => {
        const accounts = [
            {
                parsed: { amount: bn(50) },
                compressedAccount: { lamports: bn(5) },
            },
            {
                parsed: { amount: bn(30) },
                compressedAccount: { lamports: bn(3) },
            },
            {
                parsed: { amount: bn(25) },
                compressedAccount: { lamports: bn(2) },
            },
        ] as ParsedTokenAccount[];
        const transferAmount = bn(100);
        const maxInputs = 2;
        const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
            selectSmartCompressedTokenAccountsForTransferOrPartial(
                accounts,
                transferAmount,
                maxInputs,
            );

        expect(selectedAccounts.length).toBe(2);
        expect(total.eq(bn(80))).toBe(true);
        expect(totalLamports!.eq(bn(8))).toBe(true);
        expect(maxPossibleAmount.eq(bn(80))).toBe(true);
    });
});
