import { describe, expect, it } from 'vitest';
import { AccountState } from '@solana/spl-token';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createAtaInstructions,
    createFreezeInstructions,
    createLoadInstructions,
    createThawInstructions,
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
            await createLoadInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                owner: owner.publicKey,
                mint: fixture.mint,
            }),
            [owner],
        );

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createFreezeInstructions({
                tokenAccount,
                mint: fixture.mint,
                freezeAuthority: fixture.freezeAuthority!.publicKey,
            }),
            [fixture.freezeAuthority!],
        );

        expect(await getHotState(fixture.rpc, tokenAccount)).toBe(
            AccountState.Frozen,
        );

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            await createThawInstructions({
                tokenAccount,
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
