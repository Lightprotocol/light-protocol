import { PublicKey } from '@solana/web3.js';

import BN from 'bn.js';
import { getParsedEvents } from './get-parsed-events';
import { Rpc } from '../../rpc';
import {
    CompressedAccountWithMerkleContext,
    bn,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
    TreeType,
    StateTreeInfo,
} from '../../state';

/**
 * Get the queue for a given tree
 *
 * @param info - The active state tree addresses
 * @param tree - The tree to get the queue for
 * @returns The queue for the given tree, or throws an error if not found
 */
export function getQueueForTree(
    info: StateTreeInfo[],
    tree: PublicKey,
): { queue: PublicKey; treeType: TreeType; tree: PublicKey } {
    const index = info.findIndex(t => t.tree.equals(tree));

    if (index !== -1) {
        const { queue, treeType } = info[index];
        if (!queue) {
            throw new Error('Queue must not be null for state tree');
        }
        return { queue, treeType, tree: info[index].tree };
    }

    // test-rpc indexes queue as tree.
    const indexV2 = info.findIndex(
        t => t.queue && t.queue.equals(tree) && t.treeType === TreeType.StateV2,
    );
    if (indexV2 !== -1) {
        const {
            queue: actualQueue,
            treeType,
            tree: actualTree,
        } = info[indexV2];
        if (!actualQueue) {
            throw new Error('Queue must not be null for state tree');
        }

        return { queue: actualQueue, treeType, tree: actualTree };
    }

    throw new Error(
        `No associated queue found for tree. Please set activeStateTreeInfos with latest Tree accounts. If you use custom state trees, set manually. tree: ${tree.toBase58()}`,
    );
}

export async function getCompressedAccountsByOwnerTest(
    rpc: Rpc,
    owner: PublicKey,
) {
    const unspentAccounts = await getCompressedAccountsForTest(rpc);
    const byOwner = unspentAccounts.filter(acc => acc.owner.equals(owner));
    return byOwner;
}

export async function getCompressedAccountByHashTest(
    rpc: Rpc,
    hash: BN,
): Promise<CompressedAccountWithMerkleContext | undefined> {
    const unspentAccounts = await getCompressedAccountsForTest(rpc);
    return unspentAccounts.find(acc => bn(acc.hash).eq(hash));
}

export async function getMultipleCompressedAccountsByHashTest(
    rpc: Rpc,
    hashes: BN[],
): Promise<CompressedAccountWithMerkleContext[]> {
    const unspentAccounts = await getCompressedAccountsForTest(rpc);
    return unspentAccounts
        .filter(acc => hashes.some(hash => bn(acc.hash).eq(hash)))
        .sort((a, b) => b.leafIndex - a.leafIndex);
}

/// Returns all unspent compressed accounts
async function getCompressedAccountsForTest(rpc: Rpc) {
    const events = (await getParsedEvents(rpc)).reverse();
    const allOutputAccounts: CompressedAccountWithMerkleContext[] = [];
    const allInputAccountHashes: BN[] = [];
    const ctxs = await rpc.getCachedActiveStateTreeInfos();

    for (const event of events) {
        for (
            let index = 0;
            index < event.outputCompressedAccounts.length;
            index++
        ) {
            const smt =
                event.pubkeyArray[
                    event.outputCompressedAccounts[index].merkleTreeIndex
                ];

            // In test-rpc we can do this with a static set of trees because it's local-only.
            const { queue, treeType, tree } = getQueueForTree(
                ctxs,
                new PublicKey(smt),
            );

            const account = event.outputCompressedAccounts[index];
            const merkleContext: MerkleContext = {
                merkleTree: tree,
                queue: queue,
                hash: event.outputCompressedAccountHashes[index],
                leafIndex: event.outputLeafIndices[index],
                treeType,
                proveByIndex: treeType === TreeType.StateV2, // test-rpc always true because it's not forested.
            };

            const withCtx: CompressedAccountWithMerkleContext =
                createCompressedAccountWithMerkleContext(
                    merkleContext,
                    account.compressedAccount.owner,
                    account.compressedAccount.lamports,
                    account.compressedAccount.data ?? undefined,
                    account.compressedAccount.address ?? undefined,
                );
            allOutputAccounts.push(withCtx);
        }
        for (
            let index = 0;
            index < event.inputCompressedAccountHashes.length;
            index++
        ) {
            const hash = event.inputCompressedAccountHashes[index];
            allInputAccountHashes.push(bn(hash));
        }
    }

    const unspentAccounts = allOutputAccounts.filter(
        account =>
            !allInputAccountHashes.some(hash => hash.eq(bn(account.hash))),
    );
    const sorted = unspentAccounts.sort((a, b) => b.leafIndex - a.leafIndex);

    return sorted;
}
