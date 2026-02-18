/**
 * E2E tests for Kit v2 approve and revoke instructions against CToken accounts.
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
    createApproveInstruction,
    createRevokeInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('approve/revoke e2e (CToken)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;
    });

    it('approve delegate', async () => {
        const owner = await fundAccount(rpc);
        const delegate = await fundAccount(rpc);

        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, owner, mintAuthority, MINT_AMOUNT,
        );

        const ownerAddr = toKitAddress(owner.publicKey);
        const delegateAddr = toKitAddress(delegate.publicKey);

        const ix = createApproveInstruction({
            tokenAccount: ctokenAddress,
            delegate: delegateAddr,
            owner: ownerAddr,
            amount: 5_000n,
        });

        await sendKitInstructions(rpc, [ix], owner);

        // Verify on-chain: delegate is set
        const data = await getCTokenAccountData(rpc, ctokenPubkey);
        expect(data).not.toBeNull();
        expect(data!.hasDelegate).toBe(true);
        expect(data!.delegate).toBe(delegate.publicKey.toBase58());
        expect(data!.delegatedAmount).toBe(5_000n);
    });

    it('revoke delegate', async () => {
        const owner = await fundAccount(rpc);
        const delegate = await fundAccount(rpc);

        const { ctokenPubkey, ctokenAddress } = await createCTokenWithBalance(
            rpc, payer, mint, owner, mintAuthority, MINT_AMOUNT,
        );

        const ownerAddr = toKitAddress(owner.publicKey);
        const delegateAddr = toKitAddress(delegate.publicKey);

        // Approve first
        const approveIx = createApproveInstruction({
            tokenAccount: ctokenAddress,
            delegate: delegateAddr,
            owner: ownerAddr,
            amount: 5_000n,
        });
        await sendKitInstructions(rpc, [approveIx], owner);

        // Then revoke
        const revokeIx = createRevokeInstruction({
            tokenAccount: ctokenAddress,
            owner: ownerAddr,
        });
        await sendKitInstructions(rpc, [revokeIx], owner);

        // Verify on-chain: delegate is cleared
        const data = await getCTokenAccountData(rpc, ctokenPubkey);
        expect(data).not.toBeNull();
        expect(data!.hasDelegate).toBe(false);
        expect(data!.delegate).toBeNull();
    });
});
