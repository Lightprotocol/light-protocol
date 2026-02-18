/**
 * E2E tests for Kit v2 transfer instructions against CToken accounts.
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
    createTransferInstruction,
    createTransferCheckedInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('transfer e2e (CToken)', () => {
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

    it('partial transfer creates change in source account', async () => {
        const bob = await fundAccount(rpc);
        const { ctokenPubkey: bobCtoken, ctokenAddress: bobCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, bob, mintAuthority, MINT_AMOUNT);

        const { ctokenPubkey: payerCtoken, ctokenAddress: payerCtokenAddr } =
            await createCTokenAccount(rpc, payer, payer, mint);

        const transferAmount = 3_000n;
        const bobAddr = toKitAddress(bob.publicKey);

        const ix = createTransferInstruction({
            source: bobCtokenAddr,
            destination: payerCtokenAddr,
            amount: transferAmount,
            authority: bobAddr,
        });

        await sendKitInstructions(rpc, [ix], bob);

        const bobBalance = await getCTokenBalance(rpc, bobCtoken);
        const payerBalance = await getCTokenBalance(rpc, payerCtoken);

        expect(bobBalance).toBe(MINT_AMOUNT - transferAmount);
        expect(payerBalance).toBe(transferAmount);
    });

    it('full-amount transfer', async () => {
        const alice = await fundAccount(rpc);
        const charlie = await fundAccount(rpc);

        const { ctokenPubkey: aliceCtoken, ctokenAddress: aliceCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, alice, mintAuthority, MINT_AMOUNT);

        const { ctokenPubkey: charlieCtoken, ctokenAddress: charlieCtokenAddr } =
            await createCTokenAccount(rpc, payer, charlie, mint);

        const aliceAddr = toKitAddress(alice.publicKey);

        const ix = createTransferInstruction({
            source: aliceCtokenAddr,
            destination: charlieCtokenAddr,
            amount: MINT_AMOUNT,
            authority: aliceAddr,
        });

        await sendKitInstructions(rpc, [ix], alice);

        const aliceBalance = await getCTokenBalance(rpc, aliceCtoken);
        const charlieBalance = await getCTokenBalance(rpc, charlieCtoken);

        expect(aliceBalance).toBe(0n);
        expect(charlieBalance).toBe(MINT_AMOUNT);
    });

    it('transfer checked with decimals', async () => {
        const sender = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);

        const { ctokenPubkey: senderCtoken, ctokenAddress: senderCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, sender, mintAuthority, MINT_AMOUNT);

        const { ctokenPubkey: receiverCtoken, ctokenAddress: receiverCtokenAddr } =
            await createCTokenAccount(rpc, payer, receiver, mint);

        const senderAddr = toKitAddress(sender.publicKey);

        const ix = createTransferCheckedInstruction({
            source: senderCtokenAddr,
            destination: receiverCtokenAddr,
            mint: mintAddress,
            amount: 5_000n,
            authority: senderAddr,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], sender);

        const receiverBalance = await getCTokenBalance(rpc, receiverCtoken);
        expect(receiverBalance).toBe(5_000n);
    });

    it('transfer to self', async () => {
        const user = await fundAccount(rpc);
        const { ctokenPubkey: userCtoken, ctokenAddress: userCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, user, mintAuthority, MINT_AMOUNT);

        const userAddr = toKitAddress(user.publicKey);

        const ix = createTransferInstruction({
            source: userCtokenAddr,
            destination: userCtokenAddr,
            amount: 1_000n,
            authority: userAddr,
        });

        await sendKitInstructions(rpc, [ix], user);

        const balance = await getCTokenBalance(rpc, userCtoken);
        expect(balance).toBe(MINT_AMOUNT);
    });

    it('multiple sequential transfers', async () => {
        const sender = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);

        const { ctokenPubkey: senderCtoken, ctokenAddress: senderCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, sender, mintAuthority, MINT_AMOUNT);

        const { ctokenPubkey: receiverCtoken, ctokenAddress: receiverCtokenAddr } =
            await createCTokenAccount(rpc, payer, receiver, mint);

        const senderAddr = toKitAddress(sender.publicKey);

        // First transfer
        const ix1 = createTransferInstruction({
            source: senderCtokenAddr,
            destination: receiverCtokenAddr,
            amount: 2_000n,
            authority: senderAddr,
        });
        await sendKitInstructions(rpc, [ix1], sender);

        // Second transfer
        const ix2 = createTransferInstruction({
            source: senderCtokenAddr,
            destination: receiverCtokenAddr,
            amount: 3_000n,
            authority: senderAddr,
        });
        await sendKitInstructions(rpc, [ix2], sender);

        const senderBalance = await getCTokenBalance(rpc, senderCtoken);
        const receiverBalance = await getCTokenBalance(rpc, receiverCtoken);

        expect(senderBalance).toBe(MINT_AMOUNT - 5_000n);
        expect(receiverBalance).toBe(5_000n);
    });
});
