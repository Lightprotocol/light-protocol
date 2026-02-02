import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    createRpc,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';

import { createMint, mintTo, mergeTokenAccounts } from '../../src/actions';

describe('mergeTokenAccounts', () => {
    let rpc: Rpc;
    let payer: Signer;
    let owner: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        rpc = createRpc();
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

    it('should merge all token accounts', async () => {
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
});
