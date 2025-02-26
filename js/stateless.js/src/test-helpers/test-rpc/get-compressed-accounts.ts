import { PublicKey } from '@solana/web3.js';

import BN from 'bn.js';
import { getParsedEvents } from './get-parsed-events';
import { defaultTestStateTreeAccounts } from '../../constants';
import { getQueueForTree, Rpc } from '../../rpc';
import {
    CompressedAccountWithMerkleContext,
    bn,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
    MerkleContextVersion,
} from '../../state';

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
    const ctxs = await rpc.getCachedActiveStateTreeInfo();

    for (const event of events) {
        // console.log('event.pubkeyArray', event.pubkeyArray);
        // console.log(
        //     'event.outputCompressedAccounts',
        //     event.outputCompressedAccounts,
        // );
        // console.log(
        //     'out-accounts len, mt idxs',
        //     event.outputCompressedAccounts.length,
        //     event.outputCompressedAccounts.map(acc => acc.merkleTreeIndex),
        // );
        for (
            let index = 0;
            index < event.outputCompressedAccounts.length;
            index++
        ) {
            const smt =
                event.pubkeyArray[
                    event.outputCompressedAccounts[index].merkleTreeIndex
                ];
            const queue = getQueueForTree(ctxs, new PublicKey(smt));

            const account = event.outputCompressedAccounts[index];
            const merkleContext: MerkleContext = {
                merkleTree: new PublicKey(smt),
                queue,
                hash: event.outputCompressedAccountHashes[index],
                leafIndex: event.outputLeafIndices[index],
                version: MerkleContextVersion.V1,
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
