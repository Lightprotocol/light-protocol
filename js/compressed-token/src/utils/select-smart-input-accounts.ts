import { bn, ParsedTokenAccount } from '@lightprotocol/stateless.js';
import BN from 'bn.js';
const ERROR_NO_ACCOUNTS_FOUND =
    'Could not find accounts to select for transfer.';

/**
 * Selects compressed token accounts for a transfer, ensuring one extra account
 * if possible, up to maxInputs.
 *
 * 1. Sorts accounts by amount (desc)
 * 2. Selects accounts until transfer amount is met or maxInputs is reached,
 *    attempting to add one extra account if possible.
 *
 * @param {ParsedTokenAccount[]} accounts - The list of token accounts to select from.
 * @param {BN} transferAmount - The token amount to be transferred.
 * @param {number} [maxInputs=4] - The maximum number of accounts to select. Default: 4.
 * @returns {[
 *   selectedAccounts: ParsedTokenAccount[],
 *   total: BN,
 *   totalLamports: BN | null,
 *   maxPossibleAmount: BN
 * ]} - An array containing:
 *   - selectedAccounts: The accounts selected for the transfer.
 *   - total: The total amount accumulated from the selected accounts.
 *   - totalLamports: The total lamports accumulated from the selected accounts.
 *   - maxPossibleAmount: The maximum possible amount that can be transferred considering maxInputs.
 *
 * @example
 * const accounts = [
 *   { parsed: { amount: new BN(100) }, compressedAccount: { lamports: new BN(10) } },
 *   { parsed: { amount: new BN(50) }, compressedAccount: { lamports: new BN(5) } },
 *   { parsed: { amount: new BN(25) }, compressedAccount: { lamports: new BN(2) } },
 * ];
 * const transferAmount = new BN(75);
 * const maxInputs = 2;
 *
 * const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
 *   selectSmartCompressedTokenAccountsForTransfer(accounts, transferAmount, maxInputs);
 *
 * console.log(selectedAccounts.length); // 2
 * console.log(total.toString()); // '150'
 * console.log(totalLamports!.toString()); // '15'
 * console.log(maxPossibleAmount.toString()); // '150'
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
    ] = selectSmartCompressedTokenAccountsForTransferOrPartial(
        accounts,
        transferAmount,
        maxInputs,
    );

    if (accumulatedAmount.lt(bn(transferAmount))) {
        const totalBalance = accounts.reduce(
            (acc, account) => acc.add(account.parsed.amount),
            bn(0),
        );
        if (selectedAccounts.length >= maxInputs) {
            throw new Error(
                `Account limit exceeded: max ${maxPossibleAmount.toString()} (${maxInputs} accounts) per transaction. Total balance: ${totalBalance.toString()} (${accounts.length} accounts). Consider multiple transfers to spend full balance.`,
            );
        } else {
            throw new Error(
                `Insufficient balance. Required: ${transferAmount.toString()}, available: ${totalBalance.toString()}.`,
            );
        }
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
 * Executes {@link selectMinCompressedTokenAccountsForTransfer} strategy,
 * returns partial amounts if insufficient accounts are found instead of
 * throwing an error.
 */
export function selectSmartCompressedTokenAccountsForTransferOrPartial(
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

    // we can ignore zero value accounts.
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
            // Select smallest additional account if maxInputs not reached
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
