import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { getAtaProgramId } from '../../src/v3/ata-utils';

import {
    getAssociatedTokenAddressInterface as unifiedGetAssociatedTokenAddressInterface,
    createLoadAtaInstructions as unifiedCreateLoadAtaInstructions,
} from '../../src/v3/unified';

describe('unified guards', () => {
    it('throws when unified getAssociatedTokenAddressInterface uses non c-token program', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        expect(() =>
            unifiedGetAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            ),
        ).toThrow(
            'Please derive the unified ATA from the c-token program; balances across SPL, T22, and c-token are unified under the canonical c-token ATA.',
        );
    });

    it('allows unified getAssociatedTokenAddressInterface with c-token program', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        expect(() =>
            unifiedGetAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                LIGHT_TOKEN_PROGRAM_ID,
            ),
        ).not.toThrow();
    });

    // Skip unless V2+beta - createLoadAtaInstructions is a V2-only interface method requiring beta
    it.skipIf(!featureFlags.isV2() || !featureFlags.isBeta())(
        'throws when unified createLoadAtaInstructions receives non c-token ATA',
        async () => {
            const rpc = {} as Rpc;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;

            // Derive SPL ATA using base function (not unified)
            const wrongAta = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );

            await expect(
                unifiedCreateLoadAtaInstructions(
                    rpc,
                    wrongAta,
                    owner,
                    mint,
                    owner,
                ),
            ).rejects.toThrow(
                'For wrap=true, ata must be the c-token ATA. Got spl ATA instead.',
            );
        },
    );
});
