import {
    bn,
    ParsedTokenAccount,
    TreeType,
    featureFlags,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

export const ERROR_NO_ACCOUNTS_FOUND =
    'Could not find accounts to select for transfer.';

export const ERROR_MIXED_TREE_TYPES =
    'Cannot select accounts from different tree types (V1/V2) in the same batch. Filter accounts by tree type first.';

/**
 * Options for input account selection
 */
export interface SelectInputAccountsOptions {
    /**
     * Filter accounts by tree type. If provided, only accounts in trees of
     * this type will be selected. This prevents mixed V1/V2 batches which
     * fail at proof generation.
     */
    treeType?: TreeType;
}

/**
 * Filters accounts by tree type if specified
 */
function filterByTreeType(
    accounts: ParsedTokenAccount[],
    treeType?: TreeType,
): ParsedTokenAccount[] {
    if (!treeType) return accounts;
    return accounts.filter(
        acc => acc.compressedAccount.treeInfo.treeType === treeType,
    );
}

/**
 * Validates that all selected accounts are from the same tree type.
 * Throws if mixed tree types are detected.
 * Silently skips validation if accounts don't have treeInfo (e.g. mock accounts).
 */
function validateSameTreeType(accounts: ParsedTokenAccount[]): void {
    if (accounts.length <= 1) return;

    // Skip validation if treeInfo is not available (mock accounts)
    const accountsWithTreeInfo = accounts.filter(
        acc => acc.compressedAccount?.treeInfo?.treeType !== undefined,
    );
    if (accountsWithTreeInfo.length <= 1) return;

    const firstTreeType =
        accountsWithTreeInfo[0].compressedAccount.treeInfo.treeType;
    const hasMixedTypes = accountsWithTreeInfo.some(
        acc => acc.compressedAccount.treeInfo.treeType !== firstTreeType,
    );

    if (hasMixedTypes) {
        throw new Error(ERROR_MIXED_TREE_TYPES);
    }
}

/**
 * Groups accounts by tree type for separate processing
 */
export function groupAccountsByTreeType(
    accounts: ParsedTokenAccount[],
): Map<TreeType, ParsedTokenAccount[]> {
    const groups = new Map<TreeType, ParsedTokenAccount[]>();

    for (const account of accounts) {
        const treeType = account.compressedAccount.treeInfo.treeType;
        const existing = groups.get(treeType) || [];
        existing.push(account);
        groups.set(treeType, existing);
    }

    return groups;
}

/**
 * Result of selectAccountsByPreferredTreeType
 */
export interface SelectedAccountsResult {
    /** The selected accounts (all from the same tree type) */
    accounts: ParsedTokenAccount[];
    /** The tree type of the selected accounts */
    treeType: TreeType;
    /** Total balance of selected accounts */
    totalBalance: BN;
}

/**
 * Selects accounts by preferred tree type with automatic fallback.
 *
 * In V2 mode, prefers StateV2 accounts. Falls back to StateV1 if V2
 * has insufficient balance.
 *
 * This ensures all returned accounts are from the same tree type,
 * preventing mixed V1/V2 batch proof failures.
 *
 * @param accounts All available accounts (can be mixed V1/V2)
 * @param requiredAmount Minimum amount needed (optional - if not provided, returns all from preferred type)
 * @returns Selected accounts from a single tree type
 */
export function selectAccountsByPreferredTreeType(
    accounts: ParsedTokenAccount[],
    requiredAmount?: BN,
): SelectedAccountsResult {
    const preferredTreeType = featureFlags.isV2()
        ? TreeType.StateV2
        : TreeType.StateV1;

    const accountsByTreeType = groupAccountsByTreeType(accounts);

    // Try preferred tree type first
    let selectedTreeType = preferredTreeType;
    let selectedAccounts = accountsByTreeType.get(preferredTreeType) || [];
    let totalBalance = selectedAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );

    // If insufficient balance in preferred type and requiredAmount specified, try fallback
    if (requiredAmount && totalBalance.lt(requiredAmount)) {
        const fallbackType =
            preferredTreeType === TreeType.StateV2
                ? TreeType.StateV1
                : TreeType.StateV2;
        const fallbackAccounts = accountsByTreeType.get(fallbackType) || [];
        const fallbackBalance = fallbackAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        if (fallbackBalance.gte(requiredAmount)) {
            selectedTreeType = fallbackType;
            selectedAccounts = fallbackAccounts;
            totalBalance = fallbackBalance;
        }
        // If neither type has enough, proceed with preferred type
        // and let downstream selection throw the insufficient balance error
    }

    return {
        accounts: selectedAccounts,
        treeType: selectedTreeType,
        totalBalance,
    };
}

/**
 * Selects token accounts for approval, first trying to find an exact match, then falling back to minimum selection.
 *
 * @param {ParsedTokenAccount[]} accounts - Token accounts to choose from.
 * @param {BN} approveAmount - Amount to approve.
 * @param {number} [maxInputs=4] - Max accounts to select when falling back to minimum selection.
 * @param {SelectInputAccountsOptions} [options] - Optional selection options.
 * @returns {[
 *   selectedAccounts: ParsedTokenAccount[],
 *   total: BN,
 *   totalLamports: BN | null,
 *   maxPossibleAmount: BN
 * ]} - Returns:
 *   - selectedAccounts: Accounts chosen for approval.
 *   - total: Total amount from selected accounts.
 *   - totalLamports: Total lamports from selected accounts.
 *   - maxPossibleAmount: Max approvable amount given maxInputs.
 */
export function selectTokenAccountsForApprove(
    accounts: ParsedTokenAccount[],
    approveAmount: BN,
    maxInputs: number = 4,
    options?: SelectInputAccountsOptions,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    const filteredAccounts = filterByTreeType(accounts, options?.treeType);

    // First try to find an exact match
    const exactMatch = filteredAccounts.find(account =>
        account.parsed.amount.eq(approveAmount),
    );
    if (exactMatch) {
        return [
            [exactMatch],
            exactMatch.parsed.amount,
            exactMatch.compressedAccount.lamports,
            exactMatch.parsed.amount,
        ];
    }

    // If no exact match, fall back to minimum selection
    return selectMinCompressedTokenAccountsForTransfer(
        filteredAccounts,
        approveAmount,
        maxInputs,
        options,
    );
}

/**
 * Selects the minimum number of compressed token accounts required for a
 * decompress instruction, up to a specified maximum.
 *
 * @param {ParsedTokenAccount[]} accounts   Token accounts to choose from.
 * @param {BN} amount                       Amount to decompress.
 * @param {number} [maxInputs=4]            Max accounts to select. Default
 *                                          is 4.
 * @param {SelectInputAccountsOptions} [options] - Optional selection options.
 *
 * @returns Returns selected accounts and their totals.
 */
export function selectMinCompressedTokenAccountsForDecompression(
    accounts: ParsedTokenAccount[],
    amount: BN,
    maxInputs: number = 4,
    options?: SelectInputAccountsOptions,
): {
    selectedAccounts: ParsedTokenAccount[];
    total: BN;
    totalLamports: BN | null;
    maxPossibleAmount: BN;
} {
    const [selectedAccounts, total, totalLamports, maxPossibleAmount] =
        selectMinCompressedTokenAccountsForTransfer(
            accounts,
            amount,
            maxInputs,
            options,
        );
    return { selectedAccounts, total, totalLamports, maxPossibleAmount };
}

/**
 * Selects the minimum number of compressed token accounts required for a
 * transfer or decompression instruction, up to a specified maximum.
 *
 * @param {ParsedTokenAccount[]} accounts   Token accounts to choose from.
 * @param {BN} transferAmount               Amount to transfer or decompress.
 * @param {number} [maxInputs=4]            Max accounts to select. Default
 *                                          is 4.
 * @param {SelectInputAccountsOptions} [options] - Optional selection options.
 *                                          Use treeType to filter by V1/V2.
 *
 * @returns Returns selected accounts and their totals. [
 *   selectedAccounts: ParsedTokenAccount[],
 *   total: BN,
 *   totalLamports: BN | null,
 *   maxPossibleAmount: BN
 * ]
 */
export function selectMinCompressedTokenAccountsForTransfer(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
    maxInputs: number = 4,
    options?: SelectInputAccountsOptions,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    const filteredAccounts = filterByTreeType(accounts, options?.treeType);

    const [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ] = selectMinCompressedTokenAccountsForTransferOrPartial(
        filteredAccounts,
        transferAmount,
        maxInputs,
    );

    // Validate selected accounts are all same tree type
    validateSameTreeType(selectedAccounts);

    if (accumulatedAmount.lt(bn(transferAmount))) {
        const totalBalance = filteredAccounts.reduce(
            (acc, account) => acc.add(account.parsed.amount),
            bn(0),
        );
        if (selectedAccounts.length >= maxInputs) {
            throw new Error(
                `Account limit exceeded: max ${maxPossibleAmount.toString()} (${maxInputs} accounts) per transaction. Total balance: ${totalBalance.toString()} (${filteredAccounts.length} accounts). Consider multiple transfers to spend full balance.`,
            );
        } else {
            throw new Error(
                `Insufficient balance for transfer. Required: ${transferAmount.toString()}, available: ${totalBalance.toString()}.`,
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
export function selectMinCompressedTokenAccountsForTransferOrPartial(
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

    // Max, considering maxInputs
    maxPossibleAmount = accounts
        .slice(0, maxInputs)
        .reduce((total, account) => total.add(account.parsed.amount), bn(0));

    if (accumulatedAmount.lt(bn(transferAmount))) {
        console.log(
            `Insufficient balance for transfer. Requested: ${transferAmount.toString()}, Returns max available: ${maxPossibleAmount.toString()}.`,
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
 * @param {SelectInputAccountsOptions} [options] - Optional selection options.
 *                                          Use treeType to filter by V1/V2.
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
    options?: SelectInputAccountsOptions,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
    maxPossibleAmount: BN,
] {
    const filteredAccounts = filterByTreeType(accounts, options?.treeType);

    const [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports,
        maxPossibleAmount,
    ] = selectSmartCompressedTokenAccountsForTransferOrPartial(
        filteredAccounts,
        transferAmount,
        maxInputs,
    );

    // Validate selected accounts are all same tree type
    validateSameTreeType(selectedAccounts);

    if (accumulatedAmount.lt(bn(transferAmount))) {
        const totalBalance = filteredAccounts.reduce(
            (acc, account) => acc.add(account.parsed.amount),
            bn(0),
        );
        if (selectedAccounts.length >= maxInputs) {
            throw new Error(
                `Account limit exceeded: max ${maxPossibleAmount.toString()} (${maxInputs} accounts) per transaction. Total balance: ${totalBalance.toString()} (${filteredAccounts.length} accounts). Consider multiple transfers to spend full balance.`,
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
