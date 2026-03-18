/**
 * Test that delegated transfers fail correctly.
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

describe('transferDelegatedInterface - failure cases', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let stranger: Keypair;
    let recipient: Keypair;
    let mint: PublicKey;
    let ownerAta: PublicKey;
    let recipientAta: PublicKey;

    beforeAll(async () => {
        rpc = createRpc(RPC_URL, PHOTON_URL, PROVER_URL);
        payer = Keypair.generate();
        owner = Keypair.generate();
        delegate = Keypair.generate();
        stranger = Keypair.generate();
        recipient = Keypair.generate();

        await fundAccount(rpc, payer, 10e9);
        await fundAccount(rpc, owner, 10e9);
        await fundAccount(rpc, delegate, 10e9);
        await fundAccount(rpc, stranger, 10e9);

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

        await createAtaInterface(rpc, payer, mint, owner.publicKey);
        ownerAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        await createAtaInterface(rpc, payer, mint, recipient.publicKey);
        recipientAta = getAssociatedTokenAddressInterface(
            mint,
            recipient.publicKey,
        );

        await mintToInterface(
            rpc,
            payer,
            mint,
            ownerAta,
            payer,
            1_000_000_000,
        );

        // Approve delegate for 500M
        await approveInterface(
            rpc,
            payer,
            ownerAta,
            mint,
            delegate.publicKey,
            500_000_000,
            owner,
        );
    }, 120_000);

    it('rejects transfer exceeding delegated allowance', async () => {
        await expect(
            transferDelegatedInterface(
                rpc,
                payer,
                ownerAta,
                mint,
                recipientAta,
                delegate,
                owner.publicKey,
                600_000_000, // > 500M allowance
            ),
        ).rejects.toThrow();
    }, 30_000);

    it('rejects transfer from unapproved signer', async () => {
        await expect(
            transferDelegatedInterface(
                rpc,
                payer,
                ownerAta,
                mint,
                recipientAta,
                stranger, // not approved
                owner.publicKey,
                100_000_000,
            ),
        ).rejects.toThrow();
    }, 30_000);

    it('rejects transfer after revoke', async () => {
        // Revoke first
        await revokeInterface(rpc, payer, ownerAta, mint, owner);

        await expect(
            transferDelegatedInterface(
                rpc,
                payer,
                ownerAta,
                mint,
                recipientAta,
                delegate,
                owner.publicKey,
                100_000_000,
            ),
        ).rejects.toThrow();
    }, 60_000);
});