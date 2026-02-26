/**
 * E2E tests for Kit v2 close account instruction against CToken accounts.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    createCTokenWithBalance,
    sendKitInstructions,
    getCTokenAccountData,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createCloseAccountInstruction,
    createBurnInstruction,
    LIGHT_TOKEN_RENT_SPONSOR,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('close account e2e (CToken)', () => {
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

    it('close zero-balance CToken account', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const payerAddr = toKitAddress(payer.publicKey);

        // Burn all tokens to get zero balance
        const burnIx = createBurnInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            authority: holderAddr,
            amount: MINT_AMOUNT,
        });
        await sendKitInstructions(rpc, [burnIx], holder);

        // Close the zero-balance account (rentSponsor required for compressible CToken accounts)
        const closeIx = createCloseAccountInstruction({
            tokenAccount: ctokenAddress,
            destination: payerAddr,
            owner: holderAddr,
            rentSponsor: LIGHT_TOKEN_RENT_SPONSOR,
        });
        await sendKitInstructions(rpc, [closeIx], holder);

        // Account should no longer exist (or be zeroed)
        const data = await getCTokenAccountData(rpc, ctokenPubkey);
        expect(data).toBeNull();
    });
});
