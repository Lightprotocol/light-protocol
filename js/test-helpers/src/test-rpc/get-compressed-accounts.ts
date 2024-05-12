import { PublicKey } from '@solana/web3.js';

import { BN } from '@coral-xyz/anchor';
import { getParsedEvents } from './get-parsed-events';
import { defaultTestStateTreeAccounts } from '@lightprotocol/stateless.js';
import { Rpc } from '@lightprotocol/stateless.js';
import {
    CompressedAccountWithMerkleContext,
    bn,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
} from '@lightprotocol/stateless.js';

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
export async function getCompressedAccountsForTest(rpc: Rpc) {
    const events = (await getParsedEvents(rpc)).reverse();
    const allOutputAccounts: CompressedAccountWithMerkleContext[] = [];
    const allInputAccountHashes: BN[] = [];

    for (const event of events) {
        for (
            let index = 0;
            index < event.outputCompressedAccounts.length;
            index++
        ) {
            const account = event.outputCompressedAccounts[index];
            const merkleContext: MerkleContext = {
                merkleTree: defaultTestStateTreeAccounts().merkleTree,
                nullifierQueue: defaultTestStateTreeAccounts().nullifierQueue,
                hash: event.outputCompressedAccountHashes[index],
                leafIndex: event.outputLeafIndices[index],
            };
            const withCtx: CompressedAccountWithMerkleContext =
                createCompressedAccountWithMerkleContext(
                    merkleContext,
                    account.owner,
                    account.lamports,
                    account.data ?? undefined,
                    account.address ?? undefined,
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

    return unspentAccounts;
}
