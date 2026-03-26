import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    createApproveInstruction,
    createAtaInstruction,
    createFreezeInstruction,
    createRevokeInstruction,
    createThawInstruction,
    createTransferCheckedInstruction,
} from '../../src/instructions';

describe('instruction builders', () => {
    it('creates a canonical light-token ata instruction', () => {
        const payer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            payer,
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[0].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[1].pubkey.equals(mint)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(payer)).toBe(true);
    });

    it('creates a checked transfer instruction', () => {
        const source = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const instruction = createTransferCheckedInstruction({
            source,
            destination,
            mint,
            authority,
            payer,
            amount: 42n,
            decimals: 9,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.data[0]).toBe(12);
        expect(instruction.keys[0].pubkey.equals(source)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(destination)).toBe(true);
    });

    it('creates approve, revoke, freeze, and thaw instructions', () => {
        const tokenAccount = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;

        const approve = createApproveInstruction({
            tokenAccount,
            delegate,
            owner,
            amount: 10n,
        });
        const revoke = createRevokeInstruction({
            tokenAccount,
            owner,
        });
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

        expect(approve.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(revoke.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(freeze.data[0]).toBe(10);
        expect(thaw.data[0]).toBe(11);
    });

});
