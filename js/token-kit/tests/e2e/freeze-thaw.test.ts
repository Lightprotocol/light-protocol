/**
 * E2E tests for Kit v2 freeze and thaw instructions against CToken accounts.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    createCTokenWithBalance,
    createCTokenAccount,
    sendKitInstructions,
    getCTokenAccountData,
    getCTokenBalance,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createFreezeInstruction,
    createThawInstruction,
    createTransferInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('freeze/thaw e2e (CToken)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let mintAddress: string;
    let freezeAuthority: Signer;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
        freezeAuthority = await fundAccount(rpc, 1e9);

        const created = await createTestMint(
            rpc,
            payer,
            DECIMALS,
            freezeAuthority,
        );
        mint = created.mint;
        mintAuthority = created.mintAuthority;
        mintAddress = created.mintAddress;
    });

    it('freeze account', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const freezeAddr = toKitAddress(freezeAuthority.publicKey);

        const ix = createFreezeInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            freezeAuthority: freezeAddr,
        });

        await sendKitInstructions(rpc, [ix], payer, [freezeAuthority]);

        // Verify on-chain: state = 2 (frozen)
        const data = await getCTokenAccountData(rpc, ctokenPubkey);
        expect(data).not.toBeNull();
        expect(data!.state).toBe(2);
    });

    it('thaw account', async () => {
        const holder = await fundAccount(rpc);
        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT,
        );

        const freezeAddr = toKitAddress(freezeAuthority.publicKey);

        // Freeze first
        const freezeIx = createFreezeInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [freezeIx], payer, [freezeAuthority]);

        // Then thaw
        const thawIx = createThawInstruction({
            tokenAccount: ctokenAddress,
            mint: mintAddress,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [thawIx], payer, [freezeAuthority]);

        // Verify on-chain: state = 1 (initialized, not frozen)
        const data = await getCTokenAccountData(rpc, ctokenPubkey);
        expect(data).not.toBeNull();
        expect(data!.state).toBe(1);
    });

    it('transfer after thaw succeeds', async () => {
        const holder = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);

        const { ctokenPubkey: holderCtoken, ctokenAddress: holderCtokenAddr } =
            await createCTokenWithBalance(rpc, payer, mint, holder, mintAuthority, MINT_AMOUNT);

        const { ctokenPubkey: receiverCtoken, ctokenAddress: receiverCtokenAddr } =
            await createCTokenAccount(rpc, payer, receiver, mint);

        const freezeAddr = toKitAddress(freezeAuthority.publicKey);
        const holderAddr = toKitAddress(holder.publicKey);

        // Freeze
        const freezeIx = createFreezeInstruction({
            tokenAccount: holderCtokenAddr,
            mint: mintAddress,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [freezeIx], payer, [freezeAuthority]);

        // Thaw
        const thawIx = createThawInstruction({
            tokenAccount: holderCtokenAddr,
            mint: mintAddress,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [thawIx], payer, [freezeAuthority]);

        // Transfer should succeed after thaw
        const transferIx = createTransferInstruction({
            source: holderCtokenAddr,
            destination: receiverCtokenAddr,
            amount: 5_000n,
            authority: holderAddr,
        });
        await sendKitInstructions(rpc, [transferIx], holder);

        const receiverBalance = await getCTokenBalance(rpc, receiverCtoken);
        expect(receiverBalance).toBe(5_000n);
    });
});
