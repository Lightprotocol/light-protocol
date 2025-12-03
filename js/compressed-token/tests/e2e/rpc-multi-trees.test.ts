import { describe, it, beforeAll, expect } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    bn,
    createRpc,
    TreeInfo,
    featureFlags,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo, transfer } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

const TEST_TOKEN_DECIMALS = 2;

describe('rpc-multi-trees', () => {
    let rpc: Rpc;

    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    let stateTreeInfo: TreeInfo;
    let stateTreeInfo2: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        rpc = createRpc();

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

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());

        if (featureFlags.isV2()) {
            stateTreeInfo2 = stateTreeInfo;
        } else {
            stateTreeInfo2 = (await rpc.getStateTreeInfos())[1];
        }
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        bob = await newAccountWithLamports(rpc, 1e9, 256);
        charlie = await newAccountWithLamports(rpc, 1e9, 256);

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo2,
            tokenPoolInfo,
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
        expect(
            senderAccounts[0].compressedAccount.treeInfo.tree.toBase58() ===
                stateTreeInfo2.tree.toBase58(),
        ).toBe(true);
        expect(
            receiverAccounts[0].compressedAccount.treeInfo.tree.toBase58() ===
                stateTreeInfo2.tree.toBase58(),
        ).toBe(true);
    });

    it('getCompressedTokenAccountBalance should return consistent tree and queue', async () => {
        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );
        const senderAccount = senderAccounts.items[0].compressedAccount;

        expect(senderAccount.treeInfo.tree.toBase58()).toBe(
            stateTreeInfo2.tree.toBase58(),
        );
        expect(senderAccount.treeInfo.queue.toBase58()).toBe(
            stateTreeInfo2.queue.toBase58(),
        );
    });

    it('should return both compressed token accounts in different trees', async () => {
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1042),
            stateTreeInfo,
        );

        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );
        const previousAccount = senderAccounts.items.find(
            account =>
                account.compressedAccount.treeInfo.tree.toBase58() ===
                stateTreeInfo2.tree.toBase58(),
        );

        const newlyMintedAccount = senderAccounts.items.find(
            account =>
                account.compressedAccount.treeInfo.tree.toBase58() ===
                    stateTreeInfo.tree.toBase58() &&
                account.parsed.amount.toNumber() === 1042,
        );

        expect(previousAccount).toBeDefined();
        expect(newlyMintedAccount).toBeDefined();
    });
});
