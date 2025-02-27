import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    selectAccountsByTreeType,
    decideInputAccountsToUse,
    selectInputAccountsForTransfer,
} from '../../../src/utils/select-input-accounts';
import {
    createCompressedAccountWithMerkleContext,
    createMerkleContext,
    TreeType,
} from '../../../src/state';

const owner = PublicKey.unique();
const merkleTree = PublicKey.unique();
const queue = PublicKey.unique();
const hash = new Array(32).fill(1);
const leafIndex = 0;

const merkleContext1 = createMerkleContext(
    merkleTree,
    queue,
    hash,
    leafIndex,
    TreeType.State,
    false,
);
const merkleContext2 = createMerkleContext(
    merkleTree,
    queue,
    hash,
    leafIndex,
    TreeType.BatchedState,
    false,
);
const account0 = createCompressedAccountWithMerkleContext(
    merkleContext1,
    owner,
    new BN(0),
);
const account1 = createCompressedAccountWithMerkleContext(
    merkleContext1,
    owner,
    new BN(1),
);
const account2 = createCompressedAccountWithMerkleContext(
    merkleContext2,
    owner,
    new BN(100),
);
const account3 = createCompressedAccountWithMerkleContext(
    merkleContext2,
    owner,
    new BN(50),
);
const accounts = [account0, account1, account2, account3];

describe.only('selectAccountsByTreeType', () => {
    it('should select accounts with specified tree types and sum their lamports (0)', () => {
        const { selectedAccounts, totalLamports } = selectAccountsByTreeType(
            accounts,
            [TreeType.State],
        );
        expect(selectedAccounts).toEqual([account1]);
        expect(totalLamports.toString()).toBe('1');
    });
    it('should select accounts with specified tree types and sum their lamports (2)', () => {
        const { selectedAccounts, totalLamports } = selectAccountsByTreeType(
            accounts,
            [TreeType.BatchedState],
        );

        expect(selectedAccounts).toEqual([account2, account3]);
        expect(totalLamports.toString()).toBe('150');
    });

    it('should return empty if no accounts match the tree types', () => {
        const { selectedAccounts, totalLamports } = selectAccountsByTreeType(
            accounts,
            [TreeType.BatchedAddress],
        );
        expect(selectedAccounts).toEqual([]);
        expect(totalLamports.toString()).toBe('0');
    });
});

describe.only('decideInputAccountsToUse', () => {
    it('should select accountsV1 if lamports are greater than inputLamportsV1', () => {
        const result = decideInputAccountsToUse(
            new BN(50),
            [account1],
            [account2],
        );
        expect(result.selectedAccounts).toEqual([account2]);
        expect(result.inputLamports.toString()).toBe('100');
        expect(result.discardedLamports.toString()).toBe('1');
    });

    it('should prioritize v1 accounts if lamports are equal to inputLamportsV1', () => {
        const result = decideInputAccountsToUse(
            new BN(1),
            [account1],
            [account2],
        );
        expect(result.selectedAccounts).toEqual([account1]);
        expect(result.inputLamports.toString()).toBe('1');
        expect(result.discardedLamports.toString()).toBe('100');
    });

    it('should throw an error if neither inputLamportsV1 nor inputLamportsV2 are sufficient', () => {
        expect(() =>
            decideInputAccountsToUse(new BN(500), [account1], [account2]),
        ).toThrow(
            `Neither inputLamportsV1 (1) nor inputLamportsV2 (100) are sufficient to cover the required lamports (500). Consider merging your compressed accounts before transferring lamports.`,
        );
    });

    it('should select v2 if lamports are greater than inputLamportsV1', () => {
        const result = decideInputAccountsToUse(
            new BN(80),
            [account1, account3],
            [account2],
        );
        expect(result.selectedAccounts).toEqual([account2]);
        expect(result.inputLamports.toString()).toBe('100');
        expect(result.discardedLamports.toString()).toBe('51');
    });

    it('should select multiple v2 if lamports are equal to inputLamportsV2', () => {
        const result = decideInputAccountsToUse(
            new BN(150),
            [account1],
            [account2, account3],
        );
        expect(result.selectedAccounts).toEqual([account2, account3]);
        expect(result.inputLamports.toString()).toBe('150');
        expect(result.discardedLamports.toString()).toBe('1');
    });
});

describe('selectInputAccountsForTransfer', () => {
    it('should select accountsV1 if lamports are less than or equal to inputLamportsV1', () => {
        const result = selectInputAccountsForTransfer(accounts, new BN(100));
        expect(result.selectedAccounts).toEqual([account1]);
        expect(result.inputLamports.toString()).toBe('100');
        expect(result.discardedLamports.toString()).toBe('200');
    });

    it('should select accountsV2 if lamports are less than or equal to inputLamportsV2', () => {
        const result = selectInputAccountsForTransfer(accounts, new BN(200));
        expect(result.selectedAccounts).toEqual([account2]);
        expect(result.inputLamports.toString()).toBe('200');
        expect(result.discardedLamports.toString()).toBe('100');
    });

    it('should throw an error if neither inputLamportsV1 nor inputLamportsV2 are sufficient', () => {
        expect(() =>
            selectInputAccountsForTransfer(accounts, new BN(500)),
        ).toThrow();
    });
});
