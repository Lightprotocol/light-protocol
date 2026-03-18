/**
 * E2E smoke test: approve → delegated transfer → check → revoke
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    createRpc,
    featureFlags,
    VERSION,
} from '@lightprotocol/stateless.js';
import {
    createMintInterface,
    createAtaInterface,
    getAssociatedTokenAddressInterface,
    getAtaInterface,
    approveInterface,
    revokeInterface,
    transferDelegatedInterface,
    mintToInterface,
} from '../../src';

featureFlags.version = VERSION.V2;

const RPC_URL = 'http://127.0.0.1:8899';
const PHOTON_URL = 'http://127.0.0.1:8784';
const PROVER_URL = 'http://127.0.0.1:3001';

async function fundAccount(rpc: Rpc, kp: Keypair, lamports: number) {
    const sig = await rpc.requestAirdrop(kp.publicKey, lamports);
    await rpc.confirmTransaction(sig);
}

describe('transferDelegatedInterface - e2e', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let recipient: Keypair;
    let mint: PublicKey;
    let ownerAta: PublicKey;
    let recipientAta: PublicKey;

    beforeAll(async () => {
        rpc = createRpc(RPC_URL, PHOTON_URL, PROVER_URL);
        payer = Keypair.generate();
        owner = Keypair.generate();
        delegate = Keypair.generate();
        recipient = Keypair.generate();

        await fundAccount(rpc, payer, 10e9);
        await fundAccount(rpc, owner, 10e9);
        await fundAccount(rpc, delegate, 10e9);

        // Create mint
        const mintKeypair = Keypair.generate();
        const { mint: mintPubkey } = await createMintInterface(
            rpc,
            payer,
            payer,
            null,
            9,
            mintKeypair,
        );
        mint = mintPubkey;

        // Create ATAs
        await createAtaInterface(rpc, payer, mint, owner.publicKey);
        ownerAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        await createAtaInterface(rpc, payer, mint, recipient.publicKey);
        recipientAta = getAssociatedTokenAddressInterface(
            mint,
            recipient.publicKey,
        );

        // Mint 1B tokens to owner
        await mintToInterface(rpc, payer, mint, ownerAta, payer, 1_000_000_000);
    }, 120_000);

    it('approve → delegated transfer → verify balances → revoke', async () => {
        // 1. Approve delegate for 500M
        await approveInterface(
            rpc,
            payer,
            ownerAta,
            mint,
            delegate.publicKey,
            500_000_000,
            owner,
        );

        const afterApprove = await getAtaInterface(
            rpc,
            ownerAta,
            owner.publicKey,
            mint,
        );
        expect(afterApprove.parsed.delegate?.toBase58()).toBe(
            delegate.publicKey.toBase58(),
        );
        expect(afterApprove.parsed.delegatedAmount).toBe(
            BigInt(500_000_000),
        );

        // 2. Delegate transfer 200M
        const sig = await transferDelegatedInterface(
            rpc,
            payer,
            ownerAta,
            mint,
            recipientAta,
            delegate,
            owner.publicKey,
            200_000_000,
        );
        expect(sig).toBeTruthy();

        // 3. Verify balances
        const ownerAfter = await getAtaInterface(
            rpc,
            ownerAta,
            owner.publicKey,
            mint,
        );
        expect(ownerAfter.parsed.amount).toBe(BigInt(800_000_000));
        expect(ownerAfter.parsed.delegatedAmount).toBe(BigInt(300_000_000));

        const recipientAfter = await getAtaInterface(
            rpc,
            recipientAta,
            recipient.publicKey,
            mint,
        );
        expect(recipientAfter.parsed.amount).toBe(BigInt(200_000_000));

        // 4. Revoke
        await revokeInterface(rpc, payer, ownerAta, mint, owner);

        const afterRevoke = await getAtaInterface(
            rpc,
            ownerAta,
            owner.publicKey,
            mint,
        );
        expect(afterRevoke.parsed.delegate).toBeNull();
        expect(afterRevoke.parsed.delegatedAmount).toBe(BigInt(0));
    }, 120_000);
});
