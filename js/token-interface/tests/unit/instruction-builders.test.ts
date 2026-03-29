import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
    createApproveInstruction,
    createAtaInstruction,
    createAssociatedLightTokenAccountInstruction,
    createBurnCheckedInstruction,
    createBurnInstruction,
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

    it('marks fee payer as signer when transfer authority differs', () => {
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
            amount: 1n,
            decimals: 9,
        });

        expect(instruction.keys[3].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[3].isSigner).toBe(true);
        expect(instruction.keys[3].isWritable).toBe(false);
        expect(instruction.keys[5].pubkey.equals(payer)).toBe(true);
        expect(instruction.keys[5].isSigner).toBe(true);
        expect(instruction.keys[5].isWritable).toBe(true);
    });

    it('defaults transfer payer to authority when omitted', () => {
        const source = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;

        const instruction = createTransferCheckedInstruction({
            source,
            destination,
            mint,
            authority,
            amount: 1n,
            decimals: 9,
        });

        expect(instruction.keys[3].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[3].isSigner).toBe(true);
        expect(instruction.keys[3].isWritable).toBe(true);
        expect(instruction.keys[5].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[5].isSigner).toBe(false);
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

    it('uses external fee payer in approve/revoke key metas', () => {
        const tokenAccount = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const approve = createApproveInstruction({
            tokenAccount,
            delegate,
            owner,
            amount: 9n,
            payer,
        });
        const revoke = createRevokeInstruction({
            tokenAccount,
            owner,
            payer,
        });

        expect(approve.keys[2].pubkey.equals(owner)).toBe(true);
        expect(approve.keys[2].isSigner).toBe(true);
        expect(approve.keys[2].isWritable).toBe(false);
        expect(approve.keys[4].pubkey.equals(payer)).toBe(true);
        expect(approve.keys[4].isSigner).toBe(true);
        expect(approve.keys[4].isWritable).toBe(true);

        expect(revoke.keys[1].pubkey.equals(owner)).toBe(true);
        expect(revoke.keys[1].isSigner).toBe(true);
        expect(revoke.keys[1].isWritable).toBe(false);
        expect(revoke.keys[3].pubkey.equals(payer)).toBe(true);
        expect(revoke.keys[3].isSigner).toBe(true);
        expect(revoke.keys[3].isWritable).toBe(true);
    });

    it('encodes burn and burn-checked discriminators and decimals', () => {
        const source = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const burn = createBurnInstruction({
            source,
            mint,
            authority,
            amount: 123n,
            payer,
        });
        const burnChecked = createBurnCheckedInstruction({
            source,
            mint,
            authority,
            amount: 123n,
            decimals: 9,
            payer,
        });

        expect(burn.data[0]).toBe(8);
        expect(burnChecked.data[0]).toBe(15);
        expect(burnChecked.data[9]).toBe(9);
        expect(burn.keys[4].isSigner).toBe(true);
        expect(burn.keys[2].isWritable).toBe(false);
    });

    it('creates SPL ATA instruction when SPL token program is requested', () => {
        const payer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            payer,
            owner,
            mint,
            programId: TOKEN_PROGRAM_ID,
        });

        expect(instruction.programId.equals(TOKEN_PROGRAM_ID)).toBe(false);
        expect(instruction.keys[5].pubkey.equals(TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('defaults ATA payer to owner when omitted', () => {
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[2].isSigner).toBe(true);
    });

    it('omits light-token config/rent keys when compressible config is null', () => {
        const feePayer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAssociatedLightTokenAccountInstruction({
            feePayer,
            owner,
            mint,
            compressibleConfig: null,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys).toHaveLength(5);
    });

    it('defaults associated light-token fee payer to owner when omitted', () => {
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAssociatedLightTokenAccountInstruction({
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[2].isSigner).toBe(true);
    });

});
