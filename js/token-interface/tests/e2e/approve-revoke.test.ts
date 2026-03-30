import { describe, expect, it } from 'vitest';
import { ComputeBudgetProgram, Keypair } from '@solana/web3.js';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
    createApproveInstructions,
    createRevokeInstructions,
    createTransferInstructions,
    getAtaAddress,
} from '../../src';
import {
    createApproveInstructions as createApproveInstructionsNowrap,
    createRevokeInstructions as createRevokeInstructionsNowrap,
} from '../../src/nowrap';
import {
    createMintFixture,
    getHotDelegate,
    mintCompressedToOwner,
    sendInstructions,
} from './helpers';

describe('approve and revoke instructions', () => {
    it('approves and revokes on the canonical ata', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const delegate = Keypair.generate();
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 4_000n);

        const approveInstructions = await createApproveInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 1_500n,
        });

        expect(
            approveInstructions.some(instruction =>
                instruction.programId.equals(ComputeBudgetProgram.programId),
            ),
        ).toBe(false);

        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            approveInstructions,
            [owner],
        );

        const delegated = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(delegated.delegate?.toBase58()).toBe(
            delegate.publicKey.toBase58(),
        );
        expect(delegated.delegatedAmount).toBe(1_500n);

        const revokeInstructions = await createRevokeInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        expect(
            revokeInstructions.some(instruction =>
                instruction.programId.equals(ComputeBudgetProgram.programId),
            ),
        ).toBe(false);

        await sendInstructions(fixture.rpc, fixture.payer, revokeInstructions, [
            owner,
        ]);

        const revoked = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(revoked.delegate).toBeNull();
        expect(revoked.delegatedAmount).toBe(0n);
    });

    it('defaults payer to owner when omitted', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const delegate = Keypair.generate();
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 2_000n);

        const approveInstructions = await createApproveInstructions({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 500n,
        });
        await sendInstructions(
            fixture.rpc,
            fixture.payer,
            approveInstructions,
            [owner],
        );

        const delegated = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(delegated.delegate?.toBase58()).toBe(
            delegate.publicKey.toBase58(),
        );
        expect(delegated.delegatedAmount).toBe(500n);

        const revokeInstructions = await createRevokeInstructions({
            rpc: fixture.rpc,
            owner: owner.publicKey,
            mint: fixture.mint,
        });
        await sendInstructions(fixture.rpc, fixture.payer, revokeInstructions, [
            owner,
        ]);

        const revoked = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(revoked.delegate).toBeNull();
        expect(revoked.delegatedAmount).toBe(0n);
    });

    it('dedicated nowrap path approves/revokes using existing hot account state', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const delegate = Keypair.generate();
        const tokenAccount = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 2_000n);

        // Move funds to SPL ATA so nowrap approve cannot auto-wrap.
        const stageToSpl = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: owner.publicKey,
            authority: owner.publicKey,
            recipient: owner.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            amount: 1_500n,
        });
        await sendInstructions(fixture.rpc, fixture.payer, stageToSpl, [owner]);

        const nowrapApprove = await createApproveInstructionsNowrap({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 700n,
        });
        expect(nowrapApprove.length).toBe(1);
        await sendInstructions(fixture.rpc, fixture.payer, nowrapApprove, [
            owner,
        ]);
        const nowrapDelegated = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(nowrapDelegated.delegate?.toBase58()).toBe(
            delegate.publicKey.toBase58(),
        );
        expect(nowrapDelegated.delegatedAmount).toBe(700n);

        // Canonical approve also succeeds and keeps expected delegate state.
        const canonicalApprove = await createApproveInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 700n,
        });
        await sendInstructions(fixture.rpc, fixture.payer, canonicalApprove, [
            owner,
        ]);

        const delegated = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(delegated.delegate?.toBase58()).toBe(
            delegate.publicKey.toBase58(),
        );
        expect(delegated.delegatedAmount).toBe(700n);

        // Dedicated nowrap revoke works now that hot account/delegate exist.
        const nowrapRevoke = await createRevokeInstructionsNowrap({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
        });
        await sendInstructions(fixture.rpc, fixture.payer, nowrapRevoke, [
            owner,
        ]);

        const revoked = await getHotDelegate(fixture.rpc, tokenAccount);
        expect(revoked.delegate).toBeNull();
        expect(revoked.delegatedAmount).toBe(0n);
    });
});
