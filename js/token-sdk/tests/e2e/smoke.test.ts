/**
 * Smoke test: proves the full Kit v2 instruction → on-chain pipeline works.
 *
 * 1. Create mint (legacy SDK)
 * 2. Mint compressed tokens to payer (legacy SDK)
 * 3. Build transfer instruction (Kit v2 createTransferInstruction)
 * 4. Convert to web3.js v1 instruction, build tx, send & confirm
 * 5. Verify recipient balance via indexer
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

import { createTransferInstruction } from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;
const TRANSFER_AMOUNT = 3_000n;

describe('Smoke test: Kit v2 transfer on-chain', () => {
    let rpc: Rpc;
    let payer: Signer;
    let recipient: Signer;
    let mint: any;
    let mintAuthority: Signer;

    beforeAll(async () => {
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
        recipient = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;

        await mintCompressedTokens(
            rpc,
            payer,
            mint,
            payer.publicKey,
            mintAuthority,
            MINT_AMOUNT,
        );
    });

    it('should transfer compressed tokens using Kit v2 instruction builder', async () => {
        // Verify sender has tokens
        const senderBalancePre = await getCompressedBalance(
            rpc,
            payer.publicKey,
            mint,
        );
        expect(senderBalancePre).toBe(MINT_AMOUNT);

        // Get source/dest as Kit v2 addresses
        const senderAddress = toKitAddress(payer.publicKey);
        const recipientAddress = toKitAddress(recipient.publicKey);

        // Build Kit v2 transfer instruction
        const ix = createTransferInstruction({
            source: senderAddress,
            destination: recipientAddress,
            amount: TRANSFER_AMOUNT,
            authority: senderAddress,
        });

        // Send through legacy pipeline
        await sendKitInstructions(rpc, [ix], payer);

        // Verify balances
        const senderBalancePost = await getCompressedBalance(
            rpc,
            payer.publicKey,
            mint,
        );
        const recipientBalance = await getCompressedBalance(
            rpc,
            recipient.publicKey,
            mint,
        );

        expect(senderBalancePost).toBe(MINT_AMOUNT - TRANSFER_AMOUNT);
        expect(recipientBalance).toBe(TRANSFER_AMOUNT);
    });
});
