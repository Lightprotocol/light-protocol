import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getAssociatedTokenAddress } from '../../src/read';
import {
    buildTransferInstructions,
    MultiTransactionNotSupportedError,
    createAtaInstructions,
    createFreezeInstruction,
    createThawInstruction,
    getAtaAddress,
} from '../../src';

describe('public api', () => {
    it('derives the canonical light-token ata address', () => {
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        expect(getAtaAddress({ owner, mint }).equals(
            getAssociatedTokenAddress(mint, owner),
        )).toBe(true);
    });

    it('builds one canonical ata instruction', async () => {
        const payer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instructions = await createAtaInstructions({
            payer,
            owner,
            mint,
        });

        expect(instructions).toHaveLength(1);
        expect(instructions[0].programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(
            true,
        );
    });

    it('raw freeze and thaw instructions use light-token discriminators', () => {
        const tokenAccount = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;

        const freeze = createFreezeInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        });
        const thaw = createThawInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        });

        expect(freeze.data[0]).toBe(10);
        expect(thaw.data[0]).toBe(11);
    });

    it('exposes a clear single-transaction error', () => {
        const error = new MultiTransactionNotSupportedError(
            'createLoadInstructions',
            2,
        );

        expect(error.name).toBe('MultiTransactionNotSupportedError');
        expect(error.message).toContain('single-transaction');
        expect(error.message).toContain('createLoadInstructions');
    });

    it('exports canonical transfer builder', () => {
        expect(typeof buildTransferInstructions).toBe('function');
    });
});
