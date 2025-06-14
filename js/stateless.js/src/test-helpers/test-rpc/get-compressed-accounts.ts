import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { getParsedEvents } from './get-parsed-events';
import { Rpc } from '../../rpc';
import {
    CompressedAccountWithMerkleContext,
    bn,
    MerkleContext,
    createCompressedAccountWithMerkleContextLegacy,
    TreeType,
} from '../../state';
import { getStateTreeInfoByPubkey } from '../../utils/get-state-tree-infos';

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
            const maybeTree =
                event.pubkeyArray[
                    event.outputCompressedAccounts[index].merkleTreeIndex
                ];

            const treeInfo = getStateTreeInfoByPubkey(infos, maybeTree);

            const account = event.outputCompressedAccounts[index];
            const merkleContext: MerkleContext = {
                treeInfo,
                hash: bn(event.outputCompressedAccountHashes[index]),
                leafIndex: event.outputLeafIndices[index],
                // V2 trees always have proveByIndex = true in test-rpc.
                proveByIndex: treeInfo.treeType === TreeType.StateV2,
            };
            const withCtx: CompressedAccountWithMerkleContext =
                createCompressedAccountWithMerkleContextLegacy(
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
