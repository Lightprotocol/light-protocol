/**
 * Smoke test: proves the full Kit v2 instruction → on-chain CToken pipeline works.
 *
 * 1. Create decompressed CToken mint (legacy SDK)
 * 2. Create CToken accounts and mint tokens (legacy SDK)
 * 3. Build transfer instruction (Kit v2 createTransferInstruction)
 * 4. Convert to web3.js v1 instruction, build tx, send & confirm
 * 5. Verify recipient CToken balance on-chain
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    createCTokenWithBalance,
    createCTokenAccount,
    sendKitInstructions,
    getCTokenBalance,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import { createTransferInstruction } from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;
const TRANSFER_AMOUNT = 3_000n;

describe('Smoke test: Kit v2 transfer on-chain CToken', () => {
    let rpc: Rpc;
    let payer: Signer;
    let recipient: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let payerCtoken: any;
    let payerCtokenAddress: string;
    let recipientCtoken: any;
    let recipientCtokenAddress: string;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
        recipient = await fundAccount(rpc);

        // Create decompressed CToken mint
        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;

        // Create CToken accounts and mint to payer
        const payerResult = await createCTokenWithBalance(
            rpc, payer, mint, payer, mintAuthority, MINT_AMOUNT,
        );
        payerCtoken = payerResult.ctokenPubkey;
        payerCtokenAddress = payerResult.ctokenAddress;

        // Create empty CToken account for recipient
        const recipientResult = await createCTokenAccount(
            rpc, payer, recipient, mint,
        );
        recipientCtoken = recipientResult.ctokenPubkey;
        recipientCtokenAddress = recipientResult.ctokenAddress;
    });

    it('should transfer CTokens using Kit v2 instruction builder', async () => {
        // Verify sender has tokens
        const senderBalancePre = await getCTokenBalance(rpc, payerCtoken);
        expect(senderBalancePre).toBe(MINT_AMOUNT);

        // Build Kit v2 transfer instruction
        const payerAddr = toKitAddress(payer.publicKey);
        const ix = createTransferInstruction({
            source: payerCtokenAddress,
            destination: recipientCtokenAddress,
            amount: TRANSFER_AMOUNT,
            authority: payerAddr,
        });

        // Send through legacy pipeline
        await sendKitInstructions(rpc, [ix], payer);

        // Verify balances on-chain
        const senderBalancePost = await getCTokenBalance(rpc, payerCtoken);
        const recipientBalance = await getCTokenBalance(rpc, recipientCtoken);

        expect(senderBalancePost).toBe(MINT_AMOUNT - TRANSFER_AMOUNT);
        expect(recipientBalance).toBe(TRANSFER_AMOUNT);
    });
});
