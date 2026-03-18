/**
 * E2E tests for approve/revoke on SPL and Token-2022 ATAs
 * via approveInterface and revokeInterface.
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
    createMint as createSplMint,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    getAccount,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import {
    approveInterface,
    revokeInterface,
    getAssociatedTokenAddressInterface,
} from '../../src';

featureFlags.version = VERSION.V2;

const RPC_URL = 'http://127.0.0.1:8899';
const PHOTON_URL = 'http://127.0.0.1:8784';
const PROVER_URL = 'http://127.0.0.1:3001';
const DECIMALS = 9;
const MINT_AMOUNT = 1_000_000_000n;

async function fundAccount(rpc: Rpc, kp: Keypair, lamports: number) {
    const sig = await rpc.requestAirdrop(kp.publicKey, lamports);
    await rpc.confirmTransaction(sig);
}

describe('approveInterface / revokeInterface - SPL mint', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let splMint: PublicKey;
    let ownerAta: PublicKey;

    beforeAll(async () => {
        rpc = createRpc(RPC_URL, PHOTON_URL, PROVER_URL);
        payer = Keypair.generate();
        owner = Keypair.generate();
        delegate = Keypair.generate();

        await fundAccount(rpc, payer, 10e9);
        await fundAccount(rpc, owner, 10e9);

        // Create SPL mint
        splMint = await createSplMint(
            rpc,
            payer as Keypair,
            payer.publicKey,
            null,
            DECIMALS,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Create ATA and mint tokens
        const ataInfo = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            splMint,
            owner.publicKey,
            false,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        ownerAta = ataInfo.address;

        await mintTo(
            rpc,
            payer as Keypair,
            splMint,
            ownerAta,
            payer,
            MINT_AMOUNT,
            [],
            undefined,
            TOKEN_PROGRAM_ID,
        );
    }, 120_000);

    it('approve delegate on SPL ATA', async () => {
        const sig = await approveInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            delegate.publicKey,
            500_000_000n,
            owner,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(sig).toBeTruthy();

        const account = await getAccount(rpc, ownerAta, undefined, TOKEN_PROGRAM_ID);
        expect(account.delegate?.toBase58()).toBe(delegate.publicKey.toBase58());
        expect(account.delegatedAmount).toBe(500_000_000n);
    }, 60_000);

    it('revoke delegate on SPL ATA', async () => {
        const sig = await revokeInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            owner,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(sig).toBeTruthy();

        const account = await getAccount(rpc, ownerAta, undefined, TOKEN_PROGRAM_ID);
        expect(account.delegate).toBeNull();
        expect(account.delegatedAmount).toBe(0n);
    }, 60_000);

    it('rejects approve from non-owner', async () => {
        const stranger = Keypair.generate();
        await fundAccount(rpc, stranger, 1e9);

        const strangerAta = getAssociatedTokenAddressInterface(
            splMint,
            stranger.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        // strangerAta doesn't match ownerAta, should fail with mismatch
        await expect(
            approveInterface(
                rpc,
                payer,
                ownerAta,
                splMint,
                delegate.publicKey,
                100n,
                stranger,
                undefined,
                TOKEN_PROGRAM_ID,
            ),
        ).rejects.toThrow();
    }, 60_000);
});

describe('approveInterface / revokeInterface - Token-2022 mint', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let t22Mint: PublicKey;
    let ownerAta: PublicKey;

    beforeAll(async () => {
        rpc = createRpc(RPC_URL, PHOTON_URL, PROVER_URL);
        payer = Keypair.generate();
        owner = Keypair.generate();
        delegate = Keypair.generate();

        await fundAccount(rpc, payer, 10e9);
        await fundAccount(rpc, owner, 10e9);

        // Create Token-2022 mint
        t22Mint = await createSplMint(
            rpc,
            payer as Keypair,
            payer.publicKey,
            null,
            DECIMALS,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        // Create ATA and mint tokens
        const ataInfo = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            t22Mint,
            owner.publicKey,
            false,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        ownerAta = ataInfo.address;

        await mintTo(
            rpc,
            payer as Keypair,
            t22Mint,
            ownerAta,
            payer,
            MINT_AMOUNT,
            [],
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
    }, 120_000);

    it('approve delegate on T22 ATA', async () => {
        const sig = await approveInterface(
            rpc,
            payer,
            ownerAta,
            t22Mint,
            delegate.publicKey,
            500_000_000n,
            owner,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(sig).toBeTruthy();

        const account = await getAccount(rpc, ownerAta, undefined, TOKEN_2022_PROGRAM_ID);
        expect(account.delegate?.toBase58()).toBe(delegate.publicKey.toBase58());
        expect(account.delegatedAmount).toBe(500_000_000n);
    }, 60_000);

    it('revoke delegate on T22 ATA', async () => {
        const sig = await revokeInterface(
            rpc,
            payer,
            ownerAta,
            t22Mint,
            owner,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(sig).toBeTruthy();

        const account = await getAccount(rpc, ownerAta, undefined, TOKEN_2022_PROGRAM_ID);
        expect(account.delegate).toBeNull();
        expect(account.delegatedAmount).toBe(0n);
    }, 60_000);
});
