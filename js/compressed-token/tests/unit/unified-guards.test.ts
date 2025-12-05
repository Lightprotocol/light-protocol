import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { Rpc, CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getATAProgramId } from '../../src/utils';

import {
    getAssociatedTokenAddressInterface as unifiedGetAssociatedTokenAddressInterface,
    createLoadATAInstructions as unifiedCreateLoadATAInstructions,
} from '../../src/unified';

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
                CTOKEN_PROGRAM_ID,
            ),
        ).not.toThrow();
    });

    it('throws when unified createLoadATAInstructions receives non c-token ATA', async () => {
        const rpc = {} as Rpc;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        // Derive SPL ATA using base function (not unified)
        const wrongAta = getAssociatedTokenAddressSync(
            mint,
            owner,
            false,
            TOKEN_PROGRAM_ID,
            getATAProgramId(TOKEN_PROGRAM_ID),
        );

        await expect(
            unifiedCreateLoadATAInstructions(rpc, wrongAta, owner, mint, owner),
        ).rejects.toThrow(
            'Unified loadATA expects ATA to be derived from c-token program. Derive it with getAssociatedTokenAddressInterface.',
        );
    });
});
