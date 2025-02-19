import { bn, ParsedTokenAccount } from '@lightprotocol/stateless.js';

import BN from 'bn.js';

export const ERROR_NO_ACCOUNTS_FOUND =
    'Could not find accounts to select for transfer.';

/**
 * Selects the minimal number of compressed token accounts for a transfer.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
export function selectMinCompressedTokenAccountsForTransfer(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    const [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ] = selectMinCompressedTokenAccountsForTransferIdempotent(
        accounts,
        transferAmount,
        maxInputs,
    );

    if (accumulatedAmount.lt(bn(transferAmount))) {
        throw new Error(
            `Not enough balance for transfer. Required: ${transferAmount.toString()}, available: ${accumulatedAmount.toString()}`,
        );
    }

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ];
}

/**
 * Selects the minimal number of compressed token accounts for a transfer in an idempotent manner.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
export function selectMinCompressedTokenAccountsForTransferIdempotent(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    if (accounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    let accumulatedAmount = bn(0);
    let accumulatedLamports = bn(0);
    let maxPossibleAmount = bn(0);

    const selectedAccounts: ParsedTokenAccount[] = [];

    accounts.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));

    for (const account of accounts) {
        if (selectedAccounts.length >= maxInputs) break;
        if (accumulatedAmount.gte(bn(transferAmount))) break;

        if (
            !account.parsed.amount.isZero() ||
            !account.compressedAccount.lamports.isZero()
        ) {
            accumulatedAmount = accumulatedAmount.add(account.parsed.amount);
            accumulatedLamports = accumulatedLamports.add(
                account.compressedAccount.lamports,
            );
            selectedAccounts.push(account);
        }
    }

    // Calculate the maximum possible amount considering the maxInputs constraint
    maxPossibleAmount = accounts
        .slice(0, maxInputs)
        .reduce((total, account) => total.add(account.parsed.amount), bn(0));

    if (accumulatedAmount.lt(bn(transferAmount))) {
        console.warn(
            `Insufficient balance for transfer. Requested: ${transferAmount.toString()}, available: ${accumulatedAmount.toString()}.`,
        );
    }

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ];
}

/**
 * Selects compressed token accounts for a transfer, ensuring one more account than needed to meet the transfer amount is selected, up to a cap of maxInputs.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Selects accounts until the transfer amount is met, then selects the smallest additional account if possible, or the cap is reached
 */
export function selectSmartCompressedTokenAccountsForTransfer(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    const [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ] = selectSmartCompressedTokenAccountsForTransferIdempotent(
        accounts,
        transferAmount,
        maxInputs,
    );

    if (accumulatedAmount.lt(bn(transferAmount))) {
        throw new Error(
            `Not enough balance for transfer. Required: ${transferAmount.toString()}, available: ${accumulatedAmount.toString()}`,
        );
    }

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ];
}

/**
 * Idempotent version of selecting compressed token accounts for a transfer.
 * Always picks one more account than required, up to the maxInputs cap, with the additional account being the smallest one.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Selects accounts until the transfer amount is met, then adds the smallest additional account if possible
 */
export function selectSmartCompressedTokenAccountsForTransferIdempotent(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    if (accounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    let accumulatedAmount = bn(0);
    let accumulatedLamports = bn(0);

    const selectedAccounts: ParsedTokenAccount[] = [];

    // Filter out zero accounts
    const nonZeroAccounts = accounts.filter(
        account =>
            !account.parsed.amount.isZero() ||
            !account.compressedAccount.lamports.isZero(),
    );

    nonZeroAccounts.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));

    for (const account of nonZeroAccounts) {
        if (selectedAccounts.length >= maxInputs) break;
        accumulatedAmount = accumulatedAmount.add(account.parsed.amount);
        accumulatedLamports = accumulatedLamports.add(
            account.compressedAccount.lamports,
        );
        selectedAccounts.push(account);

        if (accumulatedAmount.gte(bn(transferAmount))) {
            // Select the smallest additional account if possible
            const remainingAccounts = nonZeroAccounts.slice(
                selectedAccounts.length,
            );
            if (remainingAccounts.length > 0) {
                const smallestAccount = remainingAccounts.reduce((min, acc) =>
                    acc.parsed.amount.lt(min.parsed.amount) ? acc : min,
                );
                if (selectedAccounts.length < maxInputs) {
                    selectedAccounts.push(smallestAccount);
                    accumulatedAmount = accumulatedAmount.add(
                        smallestAccount.parsed.amount,
                    );
                    accumulatedLamports = accumulatedLamports.add(
                        smallestAccount.compressedAccount.lamports,
                    );
                }
            }
            break;
        }
    }

    const maxPossibleAmount = nonZeroAccounts
        .slice(0, maxInputs)
        .reduce((max, account) => max.add(account.parsed.amount), bn(0));

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ];
}
