import BN from 'bn.js';

import { CompressedAccountWithMerkleContext } from '../../state';

import { bn } from '../../state';

/**
 * Selects the minimal number of compressed SOL accounts for a transfer.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
export function selectMinCompressedSolAccountsForTransfer(
    accounts: CompressedAccountWithMerkleContext[],
    transferLamports: BN | number,
): [selectedAccounts: CompressedAccountWithMerkleContext[], total: BN] {
    let accumulatedLamports = bn(0);
    transferLamports = bn(transferLamports);

    const selectedAccounts: CompressedAccountWithMerkleContext[] = [];

    accounts.sort((a, b) => b.lamports.cmp(a.lamports));

    for (const account of accounts) {
        if (accumulatedLamports.gte(bn(transferLamports))) break;
        accumulatedLamports = accumulatedLamports.add(account.lamports);
        selectedAccounts.push(account);
    }

    if (accumulatedLamports.lt(bn(transferLamports))) {
        throw new Error(
            `Insufficient balance for transfer. Required: ${transferLamports.toString()}, available: ${accumulatedLamports.toString()}`,
        );
    }

    return [selectedAccounts, accumulatedLamports];
}
