/**
 * E2E tests for buildTransferInstruction.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    mintCompressedTokens,
    sendKitInstructions,
    getCompressedBalance,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import { PhotonIndexer, buildTransferInstruction } from '../../src/index.js';

const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('buildTransferInstruction e2e', () => {
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

    it('build + send transfer instruction via real indexer', async () => {
        const recipient = await fundAccount(rpc);
        const ownerAddr = toKitAddress(payer.publicKey);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);

        const transferAmount = 3_000n;

        const result = await buildTransferInstruction(indexer, {
            owner: ownerAddr,
            mint: mintAddr,
            destination: recipientAddr,
            amount: transferAmount,
            authority: ownerAddr,
        });

        expect(result.instructions.length).toBeGreaterThan(0);
        expect(result.inputs.length).toBeGreaterThan(0);
        expect(result.proof).toBeDefined();

        // Send the instruction
        await sendKitInstructions(rpc, result.instructions, payer);

        // Verify balances
        const recipientBalance = await getCompressedBalance(
            rpc, recipient.publicKey, mint,
        );
        expect(recipientBalance).toBe(transferAmount);

        const payerBalance = await getCompressedBalance(
            rpc, payer.publicKey, mint,
        );
        expect(payerBalance).toBe(MINT_AMOUNT - transferAmount);
    });
});
