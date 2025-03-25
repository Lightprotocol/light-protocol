import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    bn,
    createRpc,
    getTestRpc,
    pickRandomTreeAndQueue,
    defaultTestStateTreeAccounts,
    defaultTestStateTreeAccounts2,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, transfer } from '../../src/actions';

const TEST_TOKEN_DECIMALS = 2;

describe('rpc-multi-trees', () => {
    let rpc: Rpc;

    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let treeAndQueue: { tree: PublicKey; queue: PublicKey };

    beforeAll(async () => {
        rpc = createRpc();

        treeAndQueue = pickRandomTreeAndQueue(
            await rpc.getCachedActiveStateTreeInfos(),
        );

        payer = await newAccountWithLamports(rpc, 1e9, 252);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        bob = await newAccountWithLamports(rpc, 1e9, 256);
        charlie = await newAccountWithLamports(rpc, 1e9, 256);

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            treeAndQueue.tree,
        );

        // should auto land in same tree
        await transfer(rpc, payer, mint, bn(700), bob, charlie.publicKey);
    });

    it('getCompressedTokenAccountsByOwner work with random state tree', async () => {
        const senderAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, { mint })
        ).items;

        const receiverAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            })
        ).items;

        expect(senderAccounts.length).toBe(1);
        expect(receiverAccounts.length).toBe(1);
        expect(senderAccounts[0].compressedAccount.merkleTree.toBase58()).toBe(
            treeAndQueue.tree.toBase58(),
        );
        expect(
            receiverAccounts[0].compressedAccount.merkleTree.toBase58(),
        ).toBe(treeAndQueue.tree.toBase58());
    });

    it('getCompressedTokenAccountBalance should return consistent tree and queue ', async () => {
        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );
        expect(
            senderAccounts.items[0].compressedAccount.merkleTree.toBase58(),
        ).toBe(treeAndQueue.tree.toBase58());
        expect(
            senderAccounts.items[0].compressedAccount.nullifierQueue.toBase58(),
        ).toBe(treeAndQueue.queue.toBase58());
    });

    it('should return both compressed token accounts in different trees', async () => {
        const tree1 = defaultTestStateTreeAccounts().merkleTree;
        const tree2 = defaultTestStateTreeAccounts2().merkleTree2;
        const queue1 = defaultTestStateTreeAccounts().nullifierQueue;
        const queue2 = defaultTestStateTreeAccounts2().nullifierQueue2;

        const previousTree = treeAndQueue.tree;

        let otherTree: PublicKey;
        let otherQueue: PublicKey;
        if (previousTree.toBase58() === tree1.toBase58()) {
            otherTree = tree2;
            otherQueue = queue2;
        } else {
            otherTree = tree1;
            otherQueue = queue1;
        }

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1042),
            otherTree,
        );

        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );
        const previousAccount = senderAccounts.items.find(
            account =>
                account.compressedAccount.merkleTree.toBase58() ===
                previousTree.toBase58(),
        );

        const newlyMintedAccount = senderAccounts.items.find(
            account =>
                account.compressedAccount.merkleTree.toBase58() ===
                otherTree.toBase58(),
        );

        expect(previousAccount).toBeDefined();
        expect(newlyMintedAccount).toBeDefined();
        expect(newlyMintedAccount!.parsed.amount.toNumber()).toBe(1042);
    });
});
