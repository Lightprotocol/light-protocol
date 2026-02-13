/**
 * E2E tests for Kit v2 freeze and thaw instructions.
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
    createFreezeInstruction,
    createThawInstruction,
    createTransferInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('freeze/thaw e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let freezeAuthority: Signer;

    beforeAll(async () => {
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
    });

    it('freeze account', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);
        const freezeAddr = toKitAddress(freezeAuthority.publicKey);

        const ix = createFreezeInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            freezeAuthority: freezeAddr,
        });

        await sendKitInstructions(rpc, [ix], payer, [freezeAuthority]);

        // Verify account is frozen via indexer
        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            holder.publicKey,
            { mint },
        );
        const frozen = accounts.items.find(
            (a) => a.parsed.state === 'frozen',
        );
        expect(frozen).toBeDefined();
    });

    it('thaw account', async () => {
        const holder = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const mintAddr = toKitAddress(mint);
        const freezeAddr = toKitAddress(freezeAuthority.publicKey);

        // Freeze first
        const freezeIx = createFreezeInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [freezeIx], payer, [freezeAuthority]);

        // Then thaw
        const thawIx = createThawInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [thawIx], payer, [freezeAuthority]);

        // Verify account is unfrozen
        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            holder.publicKey,
            { mint },
        );
        const frozen = accounts.items.filter(
            (a) => a.parsed.state === 'frozen',
        );
        expect(frozen.length).toBe(0);
    });

    it('transfer after thaw succeeds', async () => {
        const holder = await fundAccount(rpc);
        const receiver = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, holder.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const holderAddr = toKitAddress(holder.publicKey);
        const receiverAddr = toKitAddress(receiver.publicKey);
        const mintAddr = toKitAddress(mint);
        const freezeAddr = toKitAddress(freezeAuthority.publicKey);

        // Freeze
        const freezeIx = createFreezeInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [freezeIx], payer, [freezeAuthority]);

        // Thaw
        const thawIx = createThawInstruction({
            tokenAccount: holderAddr,
            mint: mintAddr,
            freezeAuthority: freezeAddr,
        });
        await sendKitInstructions(rpc, [thawIx], payer, [freezeAuthority]);

        // Transfer should succeed after thaw
        const transferIx = createTransferInstruction({
            source: holderAddr,
            destination: receiverAddr,
            amount: 5_000n,
            authority: holderAddr,
        });
        await sendKitInstructions(rpc, [transferIx], holder);

        const receiverBalance = await getCompressedBalance(
            rpc, receiver.publicKey, mint,
        );
        expect(receiverBalance).toBe(5_000n);
    });
});
