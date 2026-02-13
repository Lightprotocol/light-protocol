/**
 * E2E tests for Kit v2 transfer instructions.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    mintCompressedTokens,
    sendKitInstructions,
    getCompressedBalance,
    getCompressedAccountCount,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createTransferInstruction,
    createTransferCheckedInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('transfer e2e', () => {
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

    it('partial transfer creates change account', async () => {
        const bob = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, bob.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const transferAmount = 3_000n;
        const bobAddress = toKitAddress(bob.publicKey);
        const payerAddress = toKitAddress(payer.publicKey);

        const ix = createTransferInstruction({
            source: bobAddress,
            destination: payerAddress,
            amount: transferAmount,
            authority: bobAddress,
        });

        await sendKitInstructions(rpc, [ix], bob);

        const bobBalance = await getCompressedBalance(rpc, bob.publicKey, mint);
        const payerBalance = await getCompressedBalance(rpc, payer.publicKey, mint);

        expect(bobBalance).toBe(MINT_AMOUNT - transferAmount);
        expect(payerBalance).toBe(transferAmount);
    });

    it('full-amount transfer (no change account)', async () => {
        const alice = await fundAccount(rpc);
        const charlie = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, alice.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const aliceAddress = toKitAddress(alice.publicKey);
        const charlieAddress = toKitAddress(charlie.publicKey);

        const ix = createTransferInstruction({
            source: aliceAddress,
            destination: charlieAddress,
            amount: MINT_AMOUNT,
            authority: aliceAddress,
        });

        await sendKitInstructions(rpc, [ix], alice);

        const aliceBalance = await getCompressedBalance(rpc, alice.publicKey, mint);
        const charlieBalance = await getCompressedBalance(rpc, charlie.publicKey, mint);

        expect(aliceBalance).toBe(0n);
        expect(charlieBalance).toBe(MINT_AMOUNT);
    });

    it('transfer checked with decimals', async () => {
        const sender = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, sender.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const senderAddr = toKitAddress(sender.publicKey);
        const receiverAddr = toKitAddress(receiver.publicKey);
        const mintAddr = toKitAddress(mint);

        const ix = createTransferCheckedInstruction({
            source: senderAddr,
            destination: receiverAddr,
            mint: mintAddr,
            amount: 5_000n,
            authority: senderAddr,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], sender);

        const receiverBalance = await getCompressedBalance(
            rpc, receiver.publicKey, mint,
        );
        expect(receiverBalance).toBe(5_000n);
    });

    it('transfer to self', async () => {
        const user = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, user.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const userAddr = toKitAddress(user.publicKey);

        const ix = createTransferInstruction({
            source: userAddr,
            destination: userAddr,
            amount: 1_000n,
            authority: userAddr,
        });

        await sendKitInstructions(rpc, [ix], user);

        const balance = await getCompressedBalance(rpc, user.publicKey, mint);
        expect(balance).toBe(MINT_AMOUNT);
    });

    it('multiple sequential transfers', async () => {
        const sender = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, sender.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const senderAddr = toKitAddress(sender.publicKey);
        const receiverAddr = toKitAddress(receiver.publicKey);

        // First transfer
        const ix1 = createTransferInstruction({
            source: senderAddr,
            destination: receiverAddr,
            amount: 2_000n,
            authority: senderAddr,
        });
        await sendKitInstructions(rpc, [ix1], sender);

        // Second transfer
        const ix2 = createTransferInstruction({
            source: senderAddr,
            destination: receiverAddr,
            amount: 3_000n,
            authority: senderAddr,
        });
        await sendKitInstructions(rpc, [ix2], sender);

        const senderBalance = await getCompressedBalance(rpc, sender.publicKey, mint);
        const receiverBalance = await getCompressedBalance(rpc, receiver.publicKey, mint);

        expect(senderBalance).toBe(MINT_AMOUNT - 5_000n);
        expect(receiverBalance).toBe(5_000n);
    });

    it('transfer with maxTopUp parameter', async () => {
        const sender = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, sender.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const senderAddr = toKitAddress(sender.publicKey);
        const receiverAddr = toKitAddress(receiver.publicKey);

        const ix = createTransferInstruction({
            source: senderAddr,
            destination: receiverAddr,
            amount: 1_000n,
            authority: senderAddr,
            maxTopUp: 5000,
        });

        await sendKitInstructions(rpc, [ix], sender);

        const receiverBalance = await getCompressedBalance(
            rpc, receiver.publicKey, mint,
        );
        expect(receiverBalance).toBe(1_000n);
    });
});
