import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import type { TokenInterfaceAccount } from '../../src';
import {
    assertAccountFrozen,
    assertAccountNotFrozen,
    getSpendableAmount,
} from '../../src/account';

function buildAccount(input: {
    owner: ReturnType<typeof Keypair.generate>['publicKey'];
    delegate?: ReturnType<typeof Keypair.generate>['publicKey'] | null;
    amount: bigint;
    delegatedAmount?: bigint;
    isFrozen?: boolean;
}): TokenInterfaceAccount {
    const mint = Keypair.generate().publicKey;
    const parsedDelegate = input.delegate ?? null;
    const delegatedAmount = input.delegatedAmount ?? 0n;
    const isFrozen = input.isFrozen ?? false;

    return {
        address: Keypair.generate().publicKey,
        owner: input.owner,
        mint,
        amount: input.amount,
        hotAmount: input.amount,
        compressedAmount: 0n,
        hasHotAccount: true,
        requiresLoad: false,
        parsed: {
            address: Keypair.generate().publicKey,
            owner: input.owner,
            mint,
            amount: input.amount,
            delegate: parsedDelegate,
            delegatedAmount,
            isInitialized: true,
            isFrozen,
        },
        compressedAccount: null,
        ignoredCompressedAccounts: [],
        ignoredCompressedAmount: 0n,
    };
}

describe('account helpers', () => {
    it('returns full amount for owner and delegated amount for delegate', () => {
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const outsider = Keypair.generate().publicKey;
        const account = buildAccount({
            owner,
            delegate,
            amount: 100n,
            delegatedAmount: 30n,
        });

        expect(getSpendableAmount(account, owner)).toBe(100n);
        expect(getSpendableAmount(account, delegate)).toBe(30n);
        expect(getSpendableAmount(account, outsider)).toBe(0n);
    });

    it('clamps delegated spendable amount to total balance', () => {
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const account = buildAccount({
            owner,
            delegate,
            amount: 20n,
            delegatedAmount: 500n,
        });

        expect(getSpendableAmount(account, delegate)).toBe(20n);
    });

    it('throws explicit frozen-state assertion errors', () => {
        const owner = Keypair.generate().publicKey;
        const frozenAccount = buildAccount({
            owner,
            amount: 1n,
            isFrozen: true,
        });
        const activeAccount = buildAccount({
            owner,
            amount: 1n,
            isFrozen: false,
        });

        expect(() => assertAccountNotFrozen(frozenAccount, 'transfer')).toThrow(
            'Account is frozen; transfer is not allowed.',
        );
        expect(() => assertAccountFrozen(activeAccount, 'thaw')).toThrow(
            'Account is not frozen; thaw is not allowed.',
        );
    });
});
