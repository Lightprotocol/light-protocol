import { describe, expect, it } from 'vitest';
import { ComputeBudgetProgram } from '@solana/web3.js';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createLoadInstructions,
    getAta,
    getAtaAddress,
} from '../../src';
import {
    createMintFixture,
    getCompressedAmounts,
    getHotBalance,
    mintCompressedToOwner,
    sendInstructions,
} from './helpers';

describe('load instructions', () => {
    it('getAta only exposes the biggest compressed balance and tracks the ignored ones', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);

        await mintCompressedToOwner(fixture, owner.publicKey, 400n);
        await mintCompressedToOwner(fixture, owner.publicKey, 300n);
        await mintCompressedToOwner(fixture, owner.publicKey, 200n);

        const account = await getAta({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(account.parsed.amount).toBe(400n);
        expect(account.compressedAmount).toBe(400n);
        expect(account.requiresLoad).toBe(true);
        expect(account.ignoredCompressedAccounts).toHaveLength(2);
        expect(account.ignoredCompressedAmount).toBe(500n);
    });

    it('loads one compressed balance per call and leaves the smaller ones untouched', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);
        await mintCompressedToOwner(fixture, owner.publicKey, 300n);
        await mintCompressedToOwner(fixture, owner.publicKey, 200n);

        const firstInstructions = await createLoadInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(firstInstructions.length).toBeGreaterThan(0);
        expect(
            firstInstructions.some(instruction =>
                instruction.programId.equals(ComputeBudgetProgram.programId),
            ),
        ).toBe(false);

        await sendInstructions(fixture.rpc, fixture.payer, firstInstructions, [
            owner,
        ]);

        expect(await getHotBalance(fixture.rpc, tokenAccount)).toBe(500n);
        expect(
            await getCompressedAmounts(
                fixture.rpc,
                owner.publicKey,
                fixture.mint,
            ),
        ).toEqual([300n, 200n]);

        const secondInstructions = await createLoadInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await sendInstructions(fixture.rpc, fixture.payer, secondInstructions, [
            owner,
        ]);

        expect(await getHotBalance(fixture.rpc, tokenAccount)).toBe(800n);
        expect(
            await getCompressedAmounts(
                fixture.rpc,
                owner.publicKey,
                fixture.mint,
            ),
        ).toEqual([200n]);
    });
});
