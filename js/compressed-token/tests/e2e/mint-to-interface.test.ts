import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    getOrCreateAssociatedTokenAccount,
    getAccount,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import { createMintInterface } from '../../src/v3/actions';
import { mintToInterface } from '../../src/v3/actions/mint-to-interface';
import { createMint } from '../../src/actions/create-mint';
import { createAssociatedCTokenAccount } from '../../src/v3/actions/create-associated-ctoken';
import { getAssociatedCTokenAddress } from '../../src/v3/derivation';
import { getAccountInterface } from '../../src/v3/get-account-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('mintToInterface - SPL Mints', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const mintKeypair = Keypair.generate();
        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;
    });

    it('should mint SPL tokens to decompressed SPL token account', async () => {
        const recipient = Keypair.generate();
        const amount = 2000;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(BigInt(amount));
    });

    it('should mint SPL tokens with bigint amount', async () => {
        const recipient = Keypair.generate();
        const amount = 1000000000n;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(amount);
    });

    it('should fail with wrong authority for SPL mint', async () => {
        const wrongAuthority = Keypair.generate();
        const recipient = Keypair.generate();

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await expect(
            mintToInterface(rpc, payer, mint, ata.address, wrongAuthority, 100),
        ).rejects.toThrow();
    });

    it('should auto-detect TOKEN_PROGRAM_ID when programId not provided', async () => {
        const recipient = Keypair.generate();
        const amount = 500;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Don't pass programId - should auto-detect
        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(BigInt(amount));
    });
});

describe('mintToInterface - Compressed Mints', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintSigner: Keypair;
    let mintAuthority: Keypair;
    let mint: PublicKey;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintSigner = Keypair.generate();
        mintAuthority = Keypair.generate();

        const decimals = 9;
        const result = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        mint = result.mint;

    });

    it('should mint compressed tokens to onchain ctoken account', async () => {
        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            mint,
        );
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipientCToken,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        // Verify the account exists and is owned by CToken program
        const accountInterface = await getAccountInterface(
            rpc,
            recipientCToken,
            'confirmed',
        );
        expect(accountInterface).toBeDefined();
        expect(accountInterface.accountInfo.owner.toString()).toBe(
            CTOKEN_PROGRAM_ID.toBase58(),
        );
        expect(accountInterface.parsed.amount).toBe(BigInt(amount));
    });

    it('should mint compressed tokens with bigint amount', async () => {
        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            mint,
        );
        const amount = 1000000000n;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipientCToken,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInterface = await getAccountInterface(
            rpc,
            recipientCToken,
            'confirmed',
        );
        expect(accountInterface.parsed.amount).toBe(amount);
    });

    it('should fail with wrong authority for compressed mint', async () => {
        const wrongAuthority = Keypair.generate();
        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            mint,
        );

        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                recipientCToken,
                wrongAuthority,
                100,
            ),
        ).rejects.toThrow();
    });

    it('should auto-detect CTOKEN_PROGRAM_ID when programId not provided', async () => {
        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            mint,
        );
        const amount = 500;

        // Don't pass programId - should auto-detect
        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipientCToken,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInterface = await getAccountInterface(
            rpc,
            recipientCToken,
            'confirmed',
        );
        expect(accountInterface.parsed.amount).toBe(BigInt(amount));
    });
});

describe('mintToInterface - Token-2022 Mints', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const mintKeypair = Keypair.generate();
        const result = await createMintInterface(
            rpc,
            payer,
            mintAuthority.publicKey,
            null,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        mint = result.mint;
    });

    it('should mint Token-2022 tokens', async () => {
        const recipient = Keypair.generate();
        const amount = 3000;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_2022_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(BigInt(amount));
    });

    it('should mint Token-2022 tokens with bigint amount', async () => {
        const recipient = Keypair.generate();
        const amount = 2000000000n;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_2022_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(amount);
    });

    it('should auto-detect TOKEN_2022_PROGRAM_ID when programId not provided', async () => {
        const recipient = Keypair.generate();
        const amount = 750;

        const ata = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            mint,
            recipient.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        // Don't pass programId - should auto-detect Token-2022
        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            ata.address,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInfo = await getAccount(
            rpc,
            ata.address,
            'confirmed',
            TOKEN_2022_PROGRAM_ID,
        );
        expect(accountInfo.amount).toBe(BigInt(amount));
    });
});

describe('mintToInterface - Edge Cases', () => {
    let rpc: Rpc;
    let payer: Signer;
    let compressedMint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const mintSigner = Keypair.generate();
        const result = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            6,
            mintSigner,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        compressedMint = result.mint;

    });

    it('should handle zero amount minting', async () => {
        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            compressedMint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            compressedMint,
        );

        const txId = await mintToInterface(
            rpc,
            payer,
            compressedMint,
            recipientCToken,
            mintAuthority,
            0,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInterface = await getAccountInterface(
            rpc,
            recipientCToken,
            'confirmed',
        );
        expect(accountInterface.parsed.amount).toBe(BigInt(0));
    });

    it('should handle payer as authority', async () => {
        const mintSigner = Keypair.generate();
        const result = await createMintInterface(
            rpc,
            payer,
            payer as Keypair,
            null,
            9,
            mintSigner,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');

        const recipient = Keypair.generate();
        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            result.mint,
        );

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            result.mint,
        );
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            result.mint,
            recipientCToken,
            payer as Keypair,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInterface = await getAccountInterface(
            rpc,
            recipientCToken,
            'confirmed',
        );
        expect(accountInterface.parsed.amount).toBe(BigInt(amount));
    });
});
