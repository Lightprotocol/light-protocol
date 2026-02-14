/**
 * E2E tests for buildCompressedTransfer.
 *
 * Requires a running local validator + indexer + prover.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    mintCompressedTokens,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import { PhotonIndexer, buildCompressedTransfer } from '../../src/index.js';
import { DISCRIMINATOR } from '@lightprotocol/token-sdk';

const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('buildCompressedTransfer e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let indexer: PhotonIndexer;

    beforeAll(async () => {
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;

        await mintCompressedTokens(
            rpc, payer, mint, payer.publicKey, mintAuthority, MINT_AMOUNT,
        );

        indexer = new PhotonIndexer(COMPRESSION_RPC);
    });

    it('builds Transfer2 instruction with loaded accounts and proof', async () => {
        const recipient = await fundAccount(rpc);
        const ownerAddr = toKitAddress(payer.publicKey);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);
        const feePayerAddr = toKitAddress(payer.publicKey);

        const transferAmount = 3_000n;

        const result = await buildCompressedTransfer(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            amount: transferAmount,
            recipientOwner: recipientAddr,
            feePayer: feePayerAddr,
        });

        // Verify the result structure
        expect(result.instruction).toBeDefined();
        expect(result.inputs.length).toBeGreaterThan(0);
        expect(result.proof).toBeDefined();
        expect(result.totalInputAmount).toBeGreaterThanOrEqual(transferAmount);

        // Verify the Transfer2 instruction
        const ix = result.instruction;
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(ix.accounts.length).toBeGreaterThanOrEqual(4);

        // Verify loaded account data
        const input = result.inputs[0];
        expect(input.tokenAccount.token.amount).toBeGreaterThanOrEqual(0n);
        expect(input.merkleContext.tree).toBeDefined();
        expect(input.merkleContext.queue).toBeDefined();
    });
});
