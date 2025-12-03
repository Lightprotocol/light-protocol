import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    getTestRpc,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';

import { createMint, mintTo, mergeTokenAccounts } from '../../src/actions';

describe('mergeTokenAccounts', () => {
    let rpc: Rpc;
    let payer: Signer;
    let owner: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                2,
                mintKeypair,
            )
        ).mint;
    });

    beforeEach(async () => {
        owner = await newAccountWithLamports(rpc, 1e9);
        // Mint multiple token accounts to the owner
        for (let i = 0; i < 5; i++) {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                stateTreeInfo,
            );
        }
    });

    it.only('should merge all token accounts', async () => {
        const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
            owner.publicKey,
            { mint },
        );
        expect(preAccounts.items.length).to.be.greaterThan(1);

        await mergeTokenAccounts(rpc, payer, mint, owner);

        const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
            owner.publicKey,
            { mint },
        );
        expect(postAccounts.items.length).to.be.lessThan(
            preAccounts.items.length,
        );
        const totalBalance = postAccounts.items.reduce(
            (sum, account) => sum.add(account.parsed.amount),
            bn(0),
        );
        expect(totalBalance.toNumber()).to.equal(500); // 5 accounts * 100 tokens each
    });

    // TODO: add coverage for this apparent edge case. not required for now though.
    it('should handle merging when there is only one account', async () => {
        try {
            await mergeTokenAccounts(rpc, payer, mint, owner);
            console.log('First merge succeeded');

            const postFirstMergeAccounts =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });
            console.log('Accounts after first merge:', postFirstMergeAccounts);
        } catch (error) {
            console.error('First merge failed:', error);
            throw error;
        }

        // Second merge attempt
        try {
            await mergeTokenAccounts(rpc, payer, mint, owner);
            console.log('Second merge succeeded');
        } catch (error) {
            console.error('Second merge failed:', error);
        }

        const finalAccounts = await rpc.getCompressedTokenAccountsByOwner(
            owner.publicKey,
            { mint },
        );
        console.log('Final accounts:', finalAccounts);
        expect(finalAccounts.items.length).to.equal(1);
    });
});
