/**
 * E2E tests: full approve → delegated transfer → revoke cycle
 * with SPL and Token-2022 mints.
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
    getAssociatedTokenAddressSync,
    mintTo,
    getAccount,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import {
    approveInterface,
    revokeInterface,
    transferInterface,
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

describe('delegated transfer - SPL mint', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let stranger: Keypair;
    let recipient: Keypair;
    let splMint: PublicKey;
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

        const ownerAtaInfo = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            splMint,
            owner.publicKey,
            false,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        ownerAta = ownerAtaInfo.address;

        // Derive recipient ATA for balance assertions (created by transferDelegated)
        recipientAta = getAssociatedTokenAddressSync(
            splMint,
            recipient.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

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

    it('approve → delegated transfer → verify → revoke', async () => {
        // Approve
        await approveInterface(
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

        // Delegated transfer (recipient wallet — ATA created internally)
        const sig = await transferInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            recipient.publicKey,
            delegate,
            200_000_000n,
            TOKEN_PROGRAM_ID,
            undefined,
            { owner: owner.publicKey, splInterfaceInfos: [] },
        );
        expect(sig).toBeTruthy();

        // Verify balances
        const ownerAccount = await getAccount(
            rpc,
            ownerAta,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(ownerAccount.amount).toBe(800_000_000n);
        expect(ownerAccount.delegatedAmount).toBe(300_000_000n);

        const recipientAccount = await getAccount(
            rpc,
            recipientAta,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(recipientAccount.amount).toBe(200_000_000n);

        // Revoke
        await revokeInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            owner,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const afterRevoke = await getAccount(
            rpc,
            ownerAta,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(afterRevoke.delegate).toBeNull();
        expect(afterRevoke.delegatedAmount).toBe(0n);
    }, 120_000);

    it('rejects transfer exceeding allowance', async () => {
        // Re-approve for next tests
        await approveInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            delegate.publicKey,
            100_000_000n,
            owner,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await expect(
            transferInterface(
                rpc,
                payer,
                ownerAta,
                splMint,
                recipient.publicKey,
                delegate,
                200_000_000n, // > 100M allowance
                TOKEN_PROGRAM_ID,
                undefined,
                { owner: owner.publicKey },
            ),
        ).rejects.toThrow();
    }, 60_000);

    it('rejects transfer from unauthorized delegate', async () => {
        await expect(
            transferInterface(
                rpc,
                payer,
                ownerAta,
                splMint,
                recipient.publicKey,
                stranger,
                50_000_000n,
                TOKEN_PROGRAM_ID,
                undefined,
                { owner: owner.publicKey },
            ),
        ).rejects.toThrow();
    }, 60_000);

    it('rejects transfer after revoke', async () => {
        await revokeInterface(
            rpc,
            payer,
            ownerAta,
            splMint,
            owner,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await expect(
            transferInterface(
                rpc,
                payer,
                ownerAta,
                splMint,
                recipient.publicKey,
                delegate,
                50_000_000n,
                TOKEN_PROGRAM_ID,
                undefined,
                { owner: owner.publicKey },
            ),
        ).rejects.toThrow();
    }, 60_000);
});

describe('delegated transfer - Token-2022 mint', () => {
    let rpc: Rpc;
    let payer: Keypair;
    let owner: Keypair;
    let delegate: Keypair;
    let recipient: Keypair;
    let t22Mint: PublicKey;
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

        const ownerAtaInfo = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            t22Mint,
            owner.publicKey,
            false,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        ownerAta = ownerAtaInfo.address;

        // Derive recipient ATA for balance assertions (created by transferDelegated)
        recipientAta = getAssociatedTokenAddressSync(
            t22Mint,
            recipient.publicKey,
            false,
            TOKEN_2022_PROGRAM_ID,
        );

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

    it('approve → delegated transfer → verify → revoke', async () => {
        // Approve
        await approveInterface(
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

        // Delegated transfer (recipient wallet — ATA created internally)
        const sig = await transferInterface(
            rpc,
            payer,
            ownerAta,
            t22Mint,
            recipient.publicKey,
            delegate,
            200_000_000n,
            TOKEN_2022_PROGRAM_ID,
            undefined,
            { owner: owner.publicKey, splInterfaceInfos: [] },
        );
        expect(sig).toBeTruthy();

        // Verify balances
        const ownerAccount = await getAccount(
            rpc,
            ownerAta,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(ownerAccount.amount).toBe(800_000_000n);
        expect(ownerAccount.delegatedAmount).toBe(300_000_000n);

        const recipientAccount = await getAccount(
            rpc,
            recipientAta,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(recipientAccount.amount).toBe(200_000_000n);

        // Revoke
        await revokeInterface(
            rpc,
            payer,
            ownerAta,
            t22Mint,
            owner,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        const afterRevoke = await getAccount(
            rpc,
            ownerAta,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(afterRevoke.delegate).toBeNull();
        expect(afterRevoke.delegatedAmount).toBe(0n);
    }, 120_000);
});
