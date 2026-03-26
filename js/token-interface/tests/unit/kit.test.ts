import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { createAtaInstruction } from '../../src/instructions';
import {
    buildTransferInstructions,
    createAtaInstructions,
    createTransferInstructionPlan,
    toKitInstructions,
} from '../../src/kit';

describe('kit adapter', () => {
    it('converts legacy instructions to kit instructions', () => {
        const instruction = createAtaInstruction({
            payer: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            mint: Keypair.generate().publicKey,
        });

        const converted = toKitInstructions([instruction]);

        expect(converted).toHaveLength(1);
        expect(converted[0]).toBeDefined();
        expect(typeof converted[0]).toBe('object');
    });

    it('wraps canonical builders for kit consumers', async () => {
        const instructions = await createAtaInstructions({
            payer: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            mint: Keypair.generate().publicKey,
        });

        expect(instructions).toHaveLength(1);
        expect(instructions[0]).toBeDefined();
    });

    it('exports transfer builder and plan builder', () => {
        expect(typeof buildTransferInstructions).toBe('function');
        expect(typeof createTransferInstructionPlan).toBe('function');
    });
});
