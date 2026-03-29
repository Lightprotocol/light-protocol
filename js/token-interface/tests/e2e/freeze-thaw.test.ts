import { describe, expect, it } from 'vitest';
import { AccountState } from '@solana/spl-token';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createAtaInstructions,
    createFreezeInstructions,
    createThawInstructions,
    createTransferInstructions,
    getAta,
    getAtaAddress,
} from '../../src';
import {
    createMintFixture,
    getHotState,
    mintCompressedToOwner,
    sendInstructions,
} from './helpers';

describe('freeze and thaw instructions', () => {
    it('freezes and thaws a loaded hot account', async () => {
        const fixture = await createMintFixture({ withFreezeAuthority: true });
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createAtaInstructions({
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
            }),
        );

        await mintCompressedToOwner(fixture, owner.publicKey, 2_500n);

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createFreezeInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [owner, fixture.freezeAuthority!],
        );

        expect(await getHotState(fixture.rpc, tokenAccount)).toBe(
            AccountState.Frozen,
        );

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createThawInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [fixture.freezeAuthority!],
        );

        expect(await getHotState(fixture.rpc, tokenAccount)).toBe(
            AccountState.Initialized,
        );
    });

    it('blocks transfers while frozen and allows transfers after thaw', async () => {
        const fixture = await createMintFixture({ withFreezeAuthority: true });
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = await newAccountWithLamports(fixture.rpc, 1e9);

        await mintCompressedToOwner(fixture, owner.publicKey, 2_500n);

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createFreezeInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [owner, fixture.freezeAuthority!],
        );

        await expect(
            createTransferInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                mint: fixture.mint,
                sourceOwner: owner.publicKey,
                authority: owner.publicKey,
                recipient: recipient.publicKey,
                amount: 100n,
            }),
        ).rejects.toThrow('Account is frozen');

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createThawInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [fixture.freezeAuthority!],
        );

        const transferInstructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: owner.publicKey,
            authority: owner.publicKey,
            recipient: recipient.publicKey,
            amount: 100n,
        });
        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            transferInstructions,
            [owner],
        );

        const recipientAta = await getAta({
            rpc: fixture.rpc,
            owner: recipient.publicKey,
            mint: fixture.mint,
        });
        expect(recipientAta.parsed.amount).toBe(100n);
    });

    it('defaults payer to owner when omitted for freeze/thaw builders', async () => {
        const fixture = await createMintFixture({ withFreezeAuthority: true });
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 1_000n);

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createFreezeInstructions({
                rpc: fixture.rpc,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [owner, fixture.freezeAuthority!],
        );
        expect(await getHotState(fixture.rpc, tokenAccount)).toBe(
            AccountState.Frozen,
        );

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createThawInstructions({
                rpc: fixture.rpc,
                owner: owner.publicKey,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [fixture.freezeAuthority!],
        );
        expect(await getHotState(fixture.rpc, tokenAccount)).toBe(
            AccountState.Initialized,
        );
    });
});
