import { describe, expect, it } from 'vitest';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createAtaInstructions,
    getAta,
    getAtaAddress,
} from '../../src';
import { createMintFixture, sendInstructions } from './helpers';

describe('ata creation and reads', () => {
    it('creates the canonical ata and reads it back', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const ata = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        const instructions = await createAtaInstructions({
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(instructions).toHaveLength(1);

        await sendInstructions(fixture.rpc, fixture.payer, instructions);

        const account = await getAta({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(account.parsed.address.toBase58()).toBe(ata.toBase58());
        expect(account.parsed.owner.toBase58()).toBe(owner.publicKey.toBase58());
        expect(account.parsed.mint.toBase58()).toBe(fixture.mint.toBase58());
        expect(account.parsed.amount).toBe(0n);
    });

    it('replays ATA creation idempotently', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);

        const instructions = await createAtaInstructions({
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await sendInstructions(fixture.rpc, fixture.payer, instructions);
        await sendInstructions(fixture.rpc, fixture.payer, instructions);

        const account = await getAta({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(account.parsed.owner.toBase58()).toBe(owner.publicKey.toBase58());
        expect(account.parsed.amount).toBe(0n);
    });
});
