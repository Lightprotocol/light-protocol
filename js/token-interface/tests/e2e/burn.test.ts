import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import { createBurnInstructions } from '../../src';
import {
    TEST_TOKEN_DECIMALS,
    createMintFixture,
    mintCompressedToOwner,
    sendInstructions,
} from './helpers';

describe('burn instructions', () => {
    it('surfaces on-chain burn failure for unsupported mint/account combinations', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);

        await mintCompressedToOwner(fixture, owner.publicKey, 1_000n);

        const burnInstructions = await createBurnInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            authority: owner.publicKey,
            amount: 300n,
        });

        await expect(
            sendInstructions(fixture.rpc, fixture.payer, burnInstructions, [owner]),
        ).rejects.toThrow('instruction modified data of an account it does not own');
    });

    it('fails checked burn with wrong mint decimals', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);

        const burnInstructions = await createBurnInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            authority: owner.publicKey,
            amount: 100n,
            decimals: TEST_TOKEN_DECIMALS + 1,
        });

        await expect(
            sendInstructions(fixture.rpc, fixture.payer, burnInstructions, [owner]),
        ).rejects.toThrow();
    });

    it('rejects burn build for signer that is neither owner nor delegate', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const unauthorized = Keypair.generate();

        await mintCompressedToOwner(fixture, owner.publicKey, 900n);

        await expect(
            createBurnInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
                authority: unauthorized.publicKey,
                amount: 250n,
            }),
        ).rejects.toThrow('Signer is not the owner or a delegate of the account.');
    });

    it('builds burn instructions when payer is omitted', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);

        const burnInstructions = await createBurnInstructions({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
            authority: owner.publicKey,
            amount: 100n,
        });

        expect(burnInstructions.length).toBeGreaterThan(0);
    });
});
