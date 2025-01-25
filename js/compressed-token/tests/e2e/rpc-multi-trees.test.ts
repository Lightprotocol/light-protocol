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
            await rpc.getCachedActiveStateTreeInfo(),
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

    it('[rpc] getCompressedTokenAccountsByOwner with 2 mints should return both mints in different trees', async () => {
        // additional mint
        const otherTree = defaultTestStateTreeAccounts().merkleTree;
        const otherQueue = defaultTestStateTreeAccounts().nullifierQueue;
        const mint2 = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
            )
        ).mint;

        await mintTo(
            rpc,
            payer,
            mint2,
            bob.publicKey,
            mintAuthority,
            bn(1042),
            otherTree,
        );

        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
        );

        // check that mint and mint2 exist in list of senderaccounts at least once
        assert.isTrue(
            senderAccounts.items.some(
                account => account.parsed.mint.toBase58() === mint.toBase58(),
            ),
        );
        assert.isTrue(
            senderAccounts.items.some(
                account => account.parsed.mint.toBase58() === mint2.toBase58(),
            ),
        );

        const newlyMintedAccount = senderAccounts.items.find(
            account => account.parsed.mint.toBase58() === mint2.toBase58(),
        );
        // consistent tree and queue
        expect(
            newlyMintedAccount!.compressedAccount.merkleTree.toBase58(),
        ).toBe(otherTree.toBase58());
        expect(
            newlyMintedAccount!.compressedAccount.nullifierQueue.toBase58(),
        ).toBe(otherQueue.toBase58());
    });
});
