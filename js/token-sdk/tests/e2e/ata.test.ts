/**
 * E2E tests for Kit v2 create associated token account instruction.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    sendKitInstructions,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createAssociatedTokenAccountIdempotentInstruction,
    deriveAssociatedTokenAddress,
    LIGHT_TOKEN_PROGRAM_ID,
} from '../../src/index.js';
import { address } from '@solana/addresses';

const DECIMALS = 2;

describe('create ATA e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;

    beforeAll(async () => {
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;
    });

    it('create ATA idempotent: derives correct address', async () => {
        const owner = await fundAccount(rpc);
        const ownerAddr = toKitAddress(owner.publicKey);
        const mintAddr = toKitAddress(mint);

        // Derive expected ATA
        const { address: expectedAta, bump } =
            await deriveAssociatedTokenAddress(ownerAddr, mintAddr);

        expect(expectedAta).toBeDefined();
        expect(bump).toBeGreaterThanOrEqual(0);
        expect(bump).toBeLessThanOrEqual(255);
    });

    it('create ATA idempotent: builds valid instruction', async () => {
        const owner = await fundAccount(rpc);
        const ownerAddr = toKitAddress(owner.publicKey);
        const mintAddr = toKitAddress(mint);
        const payerAddr = toKitAddress(payer.publicKey);

        // Use a placeholder config and sponsor for the test
        const config = address('11111111111111111111111111111111');
        const sponsor = payerAddr;

        const result = await createAssociatedTokenAccountIdempotentInstruction({
            payer: payerAddr,
            owner: ownerAddr,
            mint: mintAddr,
            compressibleConfig: config,
            rentSponsor: sponsor,
        });

        expect(result.address).toBeDefined();
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(result.instruction.accounts).toBeDefined();
        expect(result.instruction.data).toBeInstanceOf(Uint8Array);
    });
});
