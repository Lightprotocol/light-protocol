/**
 * E2E tests for Kit v2 create associated token account instruction.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    sendKitInstructions,
    getCTokenAccountData,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createAssociatedTokenAccountIdempotentInstruction,
    deriveAssociatedTokenAddress,
    LIGHT_TOKEN_PROGRAM_ID,
} from '../../src/index.js';

const DECIMALS = 2;

describe('create ATA e2e (CToken)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let mintAddress: string;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;
        mintAddress = created.mintAddress;
    });

    it('derive ATA address: deterministic and valid', async () => {
        const owner = await fundAccount(rpc);
        const ownerAddr = toKitAddress(owner.publicKey);

        const { address: expectedAta, bump } =
            await deriveAssociatedTokenAddress(ownerAddr, mintAddress);

        expect(expectedAta).toBeDefined();
        expect(bump).toBeGreaterThanOrEqual(0);
        expect(bump).toBeLessThanOrEqual(255);

        // Same inputs produce same output
        const { address: ata2 } =
            await deriveAssociatedTokenAddress(ownerAddr, mintAddress);
        expect(ata2).toBe(expectedAta);
    });

    it('create ATA idempotent: builds valid instruction', async () => {
        const owner = await fundAccount(rpc);
        const ownerAddr = toKitAddress(owner.publicKey);
        const payerAddr = toKitAddress(payer.publicKey);

        const result = await createAssociatedTokenAccountIdempotentInstruction({
            payer: payerAddr,
            owner: ownerAddr,
            mint: mintAddress,
        });

        expect(result.address).toBeDefined();
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(result.instruction.accounts).toBeDefined();
        expect(result.instruction.data).toBeInstanceOf(Uint8Array);
    });
});
