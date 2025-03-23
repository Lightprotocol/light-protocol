import { bn, ParsedTokenAccount, TreeType } from '@lightprotocol/stateless.js';
import BN from 'bn.js';

export const ERROR_NO_ACCOUNTS_FOUND =
    'Could not find accounts to select for transfer.';

/**
 * Selects the minimum number of compressed token accounts required for a transfer, up to a specified maximum.
 *
 * @param {ParsedTokenAccount[]} accounts - Token accounts to choose from.
 * @param {BN} transferAmount - Amount to transfer.
 * @param {number} [maxInputs=4] - Max accounts to select. Default is 4.
 * @returns {[
 *   selectedAccounts: ParsedTokenAccount[],
 *   total: BN,
 *   totalLamports: BN | null,
 *   maxPossibleAmount: BN
 * ]} - Returns:
 *   - selectedAccounts: Accounts chosen for transfer.
 *   - total: Total amount from selected accounts.
 *   - totalLamports: Total lamports from selected accounts.
 *   - maxPossibleAmount: Max transferable amount given maxInputs.
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
 *   selectMinCompressedTokenAccountsForTransfer(accounts, transferAmount, maxInputs);
 *
 * console.log(selectedAccounts.length); // 2
 * console.log(total.toString()); // '150'
 * console.log(totalLamports!.toString()); // '15'
 */
export function selectMinCompressedTokenAccountsForTransfer(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmountV1: BN,
    maxPossibleAmountV2: BN,
] {
    const [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmountV1,
        maxPossibleAmountV2,
    ] = selectMinCompressedTokenAccountsForTransferOrPartial(
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
                `Account limit exceeded: max ${maxPossibleAmountV1.toString()} (${selectedAccounts.filter(acc => acc.compressedAccount.treeType === TreeType.StateV1).length} V1 accounts) or ${maxPossibleAmountV2.toString()} (${selectedAccounts.filter(acc => acc.compressedAccount.treeType === TreeType.StateV2).length} V2 accounts) per transaction. Total balance: ${totalBalance.toString()} (${accounts.length} accounts). Consider multiple transfers to spend full balance.`,
            );
        } else {
            throw new Error(
                `Insufficient balance for transfer. Required: ${transferAmount.toString()}, available: ${totalBalance.toString()}.`,
            );
        }
    }

    if (
        maxPossibleAmountV1.lt(bn(transferAmount)) &&
        maxPossibleAmountV2.lt(bn(transferAmount))
    ) {
        throw new Error(
            `Neither V1 (${maxPossibleAmountV1.toString()}) nor V2 (${maxPossibleAmountV2.toString()}) accounts are sufficient to cover the required amount (${transferAmount.toString()}). Consider merging your compressed accounts before transferring.`,
        );
    }

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmountV1,
        maxPossibleAmountV2,
    ];
}

function selectAccountsFromList(
    accountList: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number,
): [ParsedTokenAccount[], BN, BN] {
    let accumulatedAmount = bn(0);
    let accumulatedLamports = bn(0);
    const selectedAccounts: ParsedTokenAccount[] = [];

    for (const account of accountList) {
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

    return [selectedAccounts, accumulatedAmount, accumulatedLamports];
}

/**
 * Executes {@link selectMinCompressedTokenAccountsForTransfer} strategy,
 * returns partial amounts if insufficient accounts are found instead of
 * throwing an error.
 */
export function selectMinCompressedTokenAccountsForTransferOrPartial(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmountV1: BN,
    maxPossibleAmountV2: BN,
] {
    if (accounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    // Separate accounts by treeType
    const accountsV1 = accounts.filter(
        account => account.compressedAccount.treeType === TreeType.StateV1,
    );
    const accountsV2 = accounts.filter(
        account => account.compressedAccount.treeType === TreeType.StateV2,
    );

    // Sort accounts by amount in descending order
    accountsV1.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));
    accountsV2.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));

    // Select accounts from V1
    let [selectedAccounts, accumulatedAmount, accumulatedLamports] =
        selectAccountsFromList(accountsV1, transferAmount, maxInputs);

    if (accumulatedAmount.lt(bn(transferAmount))) {
        const [selectedAccountsV2, accumulatedAmountV2, accumulatedLamportsV2] =
            selectAccountsFromList(accountsV2, transferAmount, maxInputs);
        if (accumulatedAmountV2.gt(accumulatedAmount)) {
            selectedAccounts = selectedAccountsV2;
            accumulatedAmount = accumulatedAmountV2;
            accumulatedLamports = accumulatedLamportsV2;
        }
    }

    // Max, considering maxInputs
    const maxPossibleAmountV1 = accountsV1
        .slice(0, maxInputs)
        .reduce((acc, account) => acc.add(account.parsed.amount), bn(0));

    const maxPossibleAmountV2 = accountsV2
        .slice(0, maxInputs)
        .reduce((acc, account) => acc.add(account.parsed.amount), bn(0));

    if (selectedAccounts.length === 0) {
        throw new Error(ERROR_NO_ACCOUNTS_FOUND);
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmountV1,
        maxPossibleAmountV2,
    ];
}
