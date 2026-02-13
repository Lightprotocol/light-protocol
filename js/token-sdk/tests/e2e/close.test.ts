/**
 * E2E tests for Kit v2 close account instruction.
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
    createCloseAccountInstruction,
    createBurnInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('close account e2e', () => {
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

    it('close zero-balance account', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);
        const payerAddr = toKitAddress(payer.publicKey);

        // Burn all tokens to get zero balance
        const burnIx = createBurnInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            authority: holderAddr,
            amount: MINT_AMOUNT,
        });
        await sendKitInstructions(rpc, [burnIx], holder);

        const balanceAfterBurn = await getCompressedBalance(
            rpc, holder.publicKey, mint,
        );
        expect(balanceAfterBurn).toBe(0n);

        // Close the zero-balance account
        const closeIx = createCloseAccountInstruction({
            tokenAccount: holderAddr,
            destination: payerAddr,
            owner: holderAddr,
        });
        await sendKitInstructions(rpc, [closeIx], holder);

        // Account count should be 0
        const count = await getCompressedAccountCount(
            rpc, holder.publicKey, mint,
        );
        expect(count).toBe(0);
    });
});
