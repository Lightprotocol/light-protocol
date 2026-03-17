/**
 * E2E tests for createTokenAccountInstruction.
 *
 * Verifies the instruction builder produces valid instructions with the
 * correct discriminator and account layout.
 *
 * Requires a running local validator + indexer + prover.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createTokenAccountInstruction,
    deriveAssociatedTokenAddress,
    LIGHT_TOKEN_PROGRAM_ID,
    DISCRIMINATOR,
} from '../../src/index.js';

const DECIMALS = 2;

describe('createTokenAccount e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAddress: string;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAddress = created.mintAddress;
    });

    it('builds valid non-compressible token account instruction', async () => {
        const owner = await fundAccount(rpc);
        const ownerAddr = toKitAddress(owner.publicKey);

        // Derive ATA address as the token account address
        const { address: tokenAccountAddress } =
            await deriveAssociatedTokenAddress(ownerAddr, mintAddress);

        const ix = createTokenAccountInstruction({
            tokenAccount: tokenAccountAddress,
            mint: mintAddress,
            owner: ownerAddr,
        });

        // Verify instruction structure
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(ix.accounts).toHaveLength(2); // token_account (writable), mint (readonly)
        expect(ix.data[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);

        // Verify account roles
        expect(ix.accounts[0].address).toBe(tokenAccountAddress);
        expect(ix.accounts[1].address).toBe(mintAddress);
    });
});
