/**
 * E2E tests for Kit v2 approve and revoke instructions.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    mintCompressedTokens,
    sendKitInstructions,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createApproveInstruction,
    createRevokeInstruction,
} from '../../src/index.js';

const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('approve/revoke e2e', () => {
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

    it('approve delegate', async () => {
        const owner = await fundAccount(rpc);
        const delegate = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, owner.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const ownerAddr = toKitAddress(owner.publicKey);
        const delegateAddr = toKitAddress(delegate.publicKey);

        const ix = createApproveInstruction({
            tokenAccount: ownerAddr,
            delegate: delegateAddr,
            owner: ownerAddr,
            amount: 5_000n,
        });

        await sendKitInstructions(rpc, [ix], owner);

        // Verify via indexer that the delegate field is set
        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            owner.publicKey,
            { mint },
        );
        const delegated = accounts.items.find(
            (a) => a.parsed.delegate !== null,
        );
        expect(delegated).toBeDefined();
    });

    it('revoke delegate', async () => {
        const owner = await fundAccount(rpc);
        const delegate = await fundAccount(rpc);
        await mintCompressedTokens(
            rpc, payer, mint, owner.publicKey, mintAuthority, MINT_AMOUNT,
        );

        const ownerAddr = toKitAddress(owner.publicKey);
        const delegateAddr = toKitAddress(delegate.publicKey);

        // Approve first
        const approveIx = createApproveInstruction({
            tokenAccount: ownerAddr,
            delegate: delegateAddr,
            owner: ownerAddr,
            amount: 5_000n,
        });
        await sendKitInstructions(rpc, [approveIx], owner);

        // Then revoke
        const revokeIx = createRevokeInstruction({
            tokenAccount: ownerAddr,
            owner: ownerAddr,
        });
        await sendKitInstructions(rpc, [revokeIx], owner);

        // Verify delegate is cleared
        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            owner.publicKey,
            { mint },
        );
        const withDelegate = accounts.items.filter(
            (a) => a.parsed.delegate !== null,
        );
        expect(withDelegate.length).toBe(0);
    });
});
