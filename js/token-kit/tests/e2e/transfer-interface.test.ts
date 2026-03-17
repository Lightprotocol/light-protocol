/**
 * E2E tests for TransferInterface (auto-routing) and requiresCompression.
 *
 * Tests light-to-light routing and cross-boundary detection.
 *
 * Requires a running local validator + indexer + prover.
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

import {
    createTransferInterfaceInstruction,
    requiresCompression,
    LIGHT_TOKEN_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('TransferInterface e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let recipient: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let mintAddress: string;
    let payerCtoken: any;
    let payerCtokenAddress: string;
    let recipientCtoken: any;
    let recipientCtokenAddress: string;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
        recipient = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;
        mintAddress = created.mintAddress;

        // Create CToken accounts with balance
        const payerResult = await createCTokenWithBalance(
            rpc, payer, mint, payer, mintAuthority, MINT_AMOUNT,
        );
        payerCtoken = payerResult.ctokenPubkey;
        payerCtokenAddress = payerResult.ctokenAddress;

        const recipientResult = await createCTokenAccount(
            rpc, payer, recipient, mint,
        );
        recipientCtoken = recipientResult.ctokenPubkey;
        recipientCtokenAddress = recipientResult.ctokenAddress;
    });

    it('light-to-light transfer via interface', async () => {
        const transferAmount = 1_500n;
        const payerAddr = toKitAddress(payer.publicKey);

        const result = createTransferInterfaceInstruction({
            sourceOwner: LIGHT_TOKEN_PROGRAM_ID,
            destOwner: LIGHT_TOKEN_PROGRAM_ID,
            source: payerCtokenAddress,
            destination: recipientCtokenAddress,
            amount: transferAmount,
            authority: payerAddr,
            mint: mintAddress,
        });

        expect(result.transferType).toBe('light-to-light');
        expect(result.instructions).toHaveLength(1);

        // Send on-chain
        await sendKitInstructions(rpc, result.instructions, payer);

        // Verify balances
        const senderBalance = await getCTokenBalance(rpc, payerCtoken);
        const recipientBalance = await getCTokenBalance(rpc, recipientCtoken);

        expect(senderBalance).toBe(MINT_AMOUNT - transferAmount);
        expect(recipientBalance).toBe(transferAmount);
    });

    it('requiresCompression detection', () => {
        // Light-to-light: no compression needed
        expect(
            requiresCompression(LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID),
        ).toBe(false);

        // Light-to-SPL: needs compression
        expect(
            requiresCompression(LIGHT_TOKEN_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID),
        ).toBe(true);

        // SPL-to-Light: needs compression
        expect(
            requiresCompression(SPL_TOKEN_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID),
        ).toBe(true);

        // SPL-to-SPL: no compression needed
        expect(
            requiresCompression(SPL_TOKEN_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID),
        ).toBe(false);
    });
});
