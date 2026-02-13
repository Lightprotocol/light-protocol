/**
 * E2E tests for Kit v2 mint-to and burn instructions.
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

import {
    createMintToInstruction,
    createMintToCheckedInstruction,
    createBurnInstruction,
    createBurnCheckedInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('mint-to e2e', () => {
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

    it('mintTo: mint compressed tokens and verify balance', async () => {
        const recipient = await fundAccount(rpc);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);
        const authorityAddr = toKitAddress(mintAuthority.publicKey);

        const ix = createMintToInstruction({
            mint: mintAddr,
            tokenAccount: recipientAddr,
            mintAuthority: authorityAddr,
            amount: MINT_AMOUNT,
        });

        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        const balance = await getCompressedBalance(
            rpc, recipient.publicKey, mint,
        );
        expect(balance).toBe(MINT_AMOUNT);
    });

    it('mintTo checked: with decimals', async () => {
        const recipient = await fundAccount(rpc);
        const recipientAddr = toKitAddress(recipient.publicKey);
        const mintAddr = toKitAddress(mint);
        const authorityAddr = toKitAddress(mintAuthority.publicKey);

        const ix = createMintToCheckedInstruction({
            mint: mintAddr,
            tokenAccount: recipientAddr,
            mintAuthority: authorityAddr,
            amount: 5_000n,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        const balance = await getCompressedBalance(
            rpc, recipient.publicKey, mint,
        );
        expect(balance).toBe(5_000n);
    });
});

describe('burn e2e', () => {
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

    it('burn: reduce balance', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);
        const burnAmount = 3_000n;

        const ix = createBurnInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            authority: holderAddr,
            amount: burnAmount,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCompressedBalance(rpc, holder.publicKey, mint);
        expect(balance).toBe(MINT_AMOUNT - burnAmount);
    });

    it('burn checked: with decimals', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);

        const ix = createBurnCheckedInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            authority: holderAddr,
            amount: 2_000n,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCompressedBalance(rpc, holder.publicKey, mint);
        expect(balance).toBe(MINT_AMOUNT - 2_000n);
    });

    it('burn full amount', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);

        const ix = createBurnInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            authority: holderAddr,
            amount: MINT_AMOUNT,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCompressedBalance(rpc, holder.publicKey, mint);
        expect(balance).toBe(0n);
    });
});
