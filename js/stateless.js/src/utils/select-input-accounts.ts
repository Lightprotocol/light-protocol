import { bn } from '../state';

import { TreeType } from '../state';

import BN from 'bn.js';
import { CompressedAccountWithMerkleContext } from '../state';
import { sumUpLamports } from '../programs';
import { validateNumbersForInclusionProof } from './validation';

/**
 * Selects compressed accounts with the specified tree types and sums up their
 * lamports.
 *
 * @param accounts List of compressed accounts with Merkle context
 * @param treeTypes Array of tree types to filter by
 * @returns An object containing the selected accounts and the total lamports
 */
export function selectAccountsByTreeType(
    accounts: CompressedAccountWithMerkleContext[],
    treeTypes: TreeType[],
): {
    selectedAccounts: CompressedAccountWithMerkleContext[];
    totalLamports: BN;
} {
    const selectedAccounts = accounts.filter(
        item => item.lamports.gt(bn(0)) && treeTypes.includes(item.treeType),
    );
    const totalLamports = sumUpLamports(selectedAccounts);
    return { selectedAccounts, totalLamports };
}

/**
 * Determines which accounts (V1 or V2) to use and which to discard based on the
 * required lamports.
 *
 * @param lamports Required lamports
 * @param inputLamportsV1 Total lamports from accounts of type V1
 * @param inputLamportsV2 Total lamports from accounts of type V2
 * @param accountsV1 Accounts of type V1
 * @param accountsV2 Accounts of type V2
 * @returns An object containing the selected and discarded accounts and their
 * lamports
 */
export function decideInputAccountsToUse(
    lamports: BN,
    inputLamportsV1: BN,
    inputLamportsV2: BN,
    accountsV1: CompressedAccountWithMerkleContext[],
    accountsV2: CompressedAccountWithMerkleContext[],
): {
    selectedAccounts: CompressedAccountWithMerkleContext[];
    inputLamports: BN;
    discardedLamports: BN;
} {
    if (lamports.gt(inputLamportsV1)) {
        return {
            selectedAccounts: accountsV1,
            inputLamports: inputLamportsV1,
            discardedLamports: inputLamportsV2,
        };
    } else {
        return {
            selectedAccounts: accountsV2,
            inputLamports: inputLamportsV2,
            discardedLamports: inputLamportsV1,
        };
    }
}

/**
 * Selects compressed accounts with the specified tree types, sums up their lamports,
 * and determines which accounts to use and which to discard based on the required lamports.
 *
 * @param accounts List of compressed accounts with Merkle context
 * @param lamports Required lamports
 * @returns An object containing the selected accounts, total input lamports, and discarded lamports
 */
export function selectInputAccountsForTransfer(
    accounts: CompressedAccountWithMerkleContext[],
    lamports: BN,
): {
    selectedAccounts: CompressedAccountWithMerkleContext[];
    inputLamports: BN;
    discardedLamports: BN;
} {
    console.log('accounts', accounts);
    const accountsV1 = accounts.filter(
        item => item.lamports.gt(bn(0)) && item.treeType === TreeType.State,
    );
    const inputLamportsV1 = sumUpLamports(accountsV1);

    const accountsV2 = accounts.filter(
        item =>
            item.lamports.gt(bn(0)) && item.treeType === TreeType.BatchedState,
    );
    const inputLamportsV2 = sumUpLamports(accountsV2);
    console.log('inputLamportsV1', inputLamportsV1.toString());
    console.log('inputLamportsV2', inputLamportsV2.toString());
    console.log('lamports', lamports.toString());
    console.log('accountsV1', accountsV1.length);
    console.log('accountsV2', accountsV2.length);
    if (lamports.lte(inputLamportsV1)) {
        validateNumbersForInclusionProof(accountsV1.length);
        return {
            selectedAccounts: accountsV1,
            inputLamports: inputLamportsV1,
            discardedLamports: inputLamportsV2,
        };
    } else if (lamports.lte(inputLamportsV2)) {
        validateNumbersForInclusionProof(accountsV2.length);
        return {
            selectedAccounts: accountsV2,
            inputLamports: inputLamportsV2,
            discardedLamports: inputLamportsV1,
        };
    } else {
        throw new Error(
            `Neither inputLamportsV1 (${inputLamportsV1.toString()}) nor inputLamportsV2 (${inputLamportsV2.toString()}) are sufficient to cover the required lamports (${lamports.toString()}). Consider merging your compressed accounts before transferring lamports.`,
        );
    }
}
