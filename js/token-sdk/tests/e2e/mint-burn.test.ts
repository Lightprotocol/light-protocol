/**
 * E2E tests for Kit v2 mint-to and burn instructions against CToken accounts.
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
    createMintToInstruction,
    createMintToCheckedInstruction,
    createBurnInstruction,
    createBurnCheckedInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('mint-to e2e (CToken)', () => {
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

    it('mintTo: mint tokens to CToken account and verify balance', async () => {
        const recipient = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenAccount(
            rpc, payer, recipient, mint,
        );

        const authorityAddr = toKitAddress(mintAuthority.publicKey);

        const ix = createMintToInstruction({
            mint: mintAddress,
            tokenAccount: ctokenAddress,
            mintAuthority: authorityAddr,
            amount: MINT_AMOUNT,
        });

        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        const balance = await getCTokenBalance(rpc, ctokenPubkey);
        expect(balance).toBe(MINT_AMOUNT);
    });

    it('mintTo checked: with decimals', async () => {
        const recipient = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenAccount(
            rpc, payer, recipient, mint,
        );

        const authorityAddr = toKitAddress(mintAuthority.publicKey);

        const ix = createMintToCheckedInstruction({
            mint: mintAddress,
            tokenAccount: ctokenAddress,
            mintAuthority: authorityAddr,
            amount: 5_000n,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        const balance = await getCTokenBalance(rpc, ctokenPubkey);
        expect(balance).toBe(5_000n);
    });
});

describe('burn e2e (CToken)', () => {
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

    it('burn: reduce balance', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const burnAmount = 3_000n;

        const ix = createBurnInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            authority: holderAddr,
            amount: burnAmount,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCTokenBalance(rpc, ctokenPubkey);
        expect(balance).toBe(MINT_AMOUNT - burnAmount);
    });

    it('burn checked: with decimals', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);

        const ix = createBurnCheckedInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            authority: holderAddr,
            amount: 2_000n,
            decimals: DECIMALS,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCTokenBalance(rpc, ctokenPubkey);
        expect(balance).toBe(MINT_AMOUNT - 2_000n);
    });

    it('burn full amount', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);

        const ix = createBurnInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            authority: holderAddr,
            amount: MINT_AMOUNT,
        });

        await sendKitInstructions(rpc, [ix], holder);

        const balance = await getCTokenBalance(rpc, ctokenPubkey);
        expect(balance).toBe(0n);
    });
});
