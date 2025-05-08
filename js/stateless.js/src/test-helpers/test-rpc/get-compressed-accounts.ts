import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { getParsedEvents } from './get-parsed-events';
import { Rpc } from '../../rpc';
import {
    CompressedAccountWithMerkleContext,
    bn,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
    StateTreeInfo,
} from '../../state';

/**
 * Get the info for a given tree or queue
 *
 * @param info          The active state tree addresses
 * @param treeOrQueue   The tree or queue to get the info for
 * @returns The info for the given tree or queue, or throws an error if not
 * found
 */
export function getStateTreeInfoByPubkey(
    treeInfos: StateTreeInfo[],
    treeOrQueue: PublicKey,
): StateTreeInfo {
    if (treeInfos.some(t => t.queue.equals(treeOrQueue)))
        throw new Error('Checking by queue not supported yet');

    const index = treeInfos.findIndex(t => t.tree.equals(treeOrQueue));

    if (index !== -1) {
        const { queue, treeType } = treeInfos[index];
        if (!queue) {
            throw new Error('Queue must not be null for state tree');
        }
        return {
            queue,
            treeType,
            tree: treeInfos[index].tree,
            cpiContext: treeInfos[index].cpiContext,
            nextTreeInfo: treeInfos[index].nextTreeInfo,
        };
    }

    throw new Error(
        `No associated StateTreeInfo found for tree or queue. Please set activeStateTreeInfos with latest Tree accounts. If you use custom state trees, set manually. Pubkey: ${treeOrQueue.toBase58()}`,
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
    const infos = await rpc.getStateTreeInfos();

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

            const treeInfo = getStateTreeInfoByPubkey(
                infos,
                new PublicKey(smt),
            );

            const account = event.outputCompressedAccounts[index];
            const merkleContext: MerkleContext = {
                treeInfo,
                hash: bn(event.outputCompressedAccountHashes[index]),
                leafIndex: event.outputLeafIndices[index],
                proveByIndex: false,
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
    unspentAccounts.sort((a, b) => b.leafIndex - a.leafIndex);

    return unspentAccounts;
}
