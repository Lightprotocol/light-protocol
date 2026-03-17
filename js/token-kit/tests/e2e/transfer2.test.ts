/**
 * E2E tests for Transfer2 instruction (compressed token transfers).
 *
 * Uses V3/stateless.js for setup (mints, compressed token accounts).
 * Uses token-kit's buildCompressedTransfer + createTransfer2Instruction for operations.
 * Verifies results via indexer.
 *
 * Requires a running local validator + indexer + prover.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createCompressedMint,
    mintCompressedTokens,
    toKitAddress,
    sendKitInstructions,
    getCompressedBalance,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    PhotonIndexer,
    buildCompressedTransfer,
    DISCRIMINATOR,
} from '../../src/index.js';

const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const DECIMALS = 2;

describe('Transfer2 e2e (compressed)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let indexer: PhotonIndexer;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createCompressedMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;

        // Mint initial tokens
        await mintCompressedTokens(
            rpc, payer, mint, payer.publicKey, mintAuthority, 10_000,
        );

        indexer = new PhotonIndexer(COMPRESSION_RPC);
    });

    it('compressed transfer: send on-chain and verify via indexer', async () => {
        const recipient = await fundAccount(rpc);
        const ownerAddr = toKitAddress(payer.publicKey);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);

        const transferAmount = 2_000n;

        const balanceBefore = await getCompressedBalance(
            rpc, payer.publicKey, mint,
        );

        const result = await buildCompressedTransfer(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            amount: transferAmount,
            recipientOwner: recipientAddr,
            feePayer: ownerAddr,
        });

        // Send on-chain
        await sendKitInstructions(rpc, [result.instruction], payer);

        // Verify balances via indexer
        const senderBalance = await getCompressedBalance(
            rpc, payer.publicKey, mint,
        );
        const recipientBalance = await getCompressedBalance(
            rpc, recipient.publicKey, mint,
        );

        expect(recipientBalance).toBe(transferAmount);
        expect(senderBalance).toBe(balanceBefore - transferAmount);
    });

    it('transfer with change: sender gets remainder back', async () => {
        const recipient = await fundAccount(rpc);
        const ownerAddr = toKitAddress(payer.publicKey);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);

        const balanceBefore = await getCompressedBalance(
            rpc, payer.publicKey, mint,
        );
        // Transfer less than total to force change output
        const transferAmount = 300n;

        const result = await buildCompressedTransfer(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            amount: transferAmount,
            recipientOwner: recipientAddr,
            feePayer: ownerAddr,
        });

        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(result.totalInputAmount).toBeGreaterThan(transferAmount);

        await sendKitInstructions(rpc, [result.instruction], payer);

        const senderBalance = await getCompressedBalance(
            rpc, payer.publicKey, mint,
        );
        const recipientBalance = await getCompressedBalance(
            rpc, recipient.publicKey, mint,
        );

        expect(recipientBalance).toBe(transferAmount);
        expect(senderBalance).toBe(balanceBefore - transferAmount);
    });

    it('multi-input transfer: consumes multiple compressed accounts', async () => {
        // Create a new mint for isolation
        const multiPayer = await fundAccount(rpc);
        const multiCreated = await createCompressedMint(
            rpc, multiPayer, DECIMALS,
        );
        const multiMint = multiCreated.mint;
        const multiAuthority = multiCreated.mintAuthority;

        // Mint 100 tokens 5 times → 5 separate compressed accounts
        for (let i = 0; i < 5; i++) {
            await mintCompressedTokens(
                rpc,
                multiPayer,
                multiMint,
                multiPayer.publicKey,
                multiAuthority,
                100,
            );
        }

        const ownerAddr = toKitAddress(multiPayer.publicKey);
        const recipient = await fundAccount(rpc);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(multiMint);

        // Transfer 400 → needs at least 4 inputs
        const transferAmount = 400n;

        const result = await buildCompressedTransfer(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            amount: transferAmount,
            recipientOwner: recipientAddr,
            feePayer: ownerAddr,
            maxInputs: 5,
        });

        expect(result.inputs.length).toBeGreaterThanOrEqual(4);

        await sendKitInstructions(rpc, [result.instruction], multiPayer);

        const recipientBalance = await getCompressedBalance(
            rpc, recipient.publicKey, multiMint,
        );
        const senderBalance = await getCompressedBalance(
            rpc, multiPayer.publicKey, multiMint,
        );

        expect(recipientBalance).toBe(transferAmount);
        expect(senderBalance).toBe(500n - transferAmount);
    });

    it('Transfer2 instruction has correct discriminator and account structure', async () => {
        const recipient = await fundAccount(rpc);
        const ownerAddr = toKitAddress(payer.publicKey);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);

        const result = await buildCompressedTransfer(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            amount: 100n,
            recipientOwner: recipientAddr,
            feePayer: ownerAddr,
        });

        // Verify Transfer2 instruction structure
        const ix = result.instruction;
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        // Path B: at least 7 system accounts + packed accounts
        expect(ix.accounts.length).toBeGreaterThanOrEqual(7);
        expect(result.proof).toBeDefined();
        expect(result.proof.proof).toBeDefined();
    });
});
