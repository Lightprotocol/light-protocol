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
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getMint,
} from '@solana/spl-token';
import { createMintInterface } from '../../src/v3/actions/create-mint-interface';
import { createTokenMetadata } from '../../src/v3/instructions';
import { getMintInterface } from '../../src/v3/get-mint-interface';
import { findMintAddress } from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('createMintInterface', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    describe('CToken (compressed) - default programId', () => {
        it('should create compressed mint with default programId', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            expect(mint.toBase58()).toBe(mintPda.toBase58());

            const { mint: fetchedMint } = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                CTOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(fetchedMint.isInitialized).toBe(true);
        });

        it('should create compressed mint with explicit CTOKEN_PROGRAM_ID', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                6,
                mintSigner,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            expect(mint.toBase58()).toBe(mintPda.toBase58());
        });

        it('should create compressed mint with freeze authority', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                freezeAuthority.publicKey,
                9,
                mintSigner,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const { mint: fetchedMint } = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                CTOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });

        it('should create compressed mint with token metadata', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const metadata = createTokenMetadata(
                'Test Token',
                'TEST',
                'https://test.com/metadata.json',
                mintAuthority.publicKey,
            );

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
                undefined,
                CTOKEN_PROGRAM_ID,
                metadata,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            expect(mint.toBase58()).toBe(mintPda.toBase58());
        });

        it('should fail when mintAuthority is not a Signer for compressed mint', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate().publicKey; // PublicKey, not Signer

            await expect(
                createMintInterface(
                    rpc,
                    payer,
                    mintAuthority, // This should fail
                    null,
                    9,
                    mintSigner,
                ),
            ).rejects.toThrow(
                'mintAuthority must be a Signer for compressed token mints',
            );
        });
    });

    describe('SPL Token (TOKEN_PROGRAM_ID)', () => {
        it('should create SPL Token mint', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey, // Can be PublicKey for SPL
                null,
                9,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            expect(mint.toBase58()).toBe(mintKeypair.publicKey.toBase58());

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(fetchedMint.isInitialized).toBe(true);
            expect(fetchedMint.decimals).toBe(9);
        });

        it('should create SPL Token mint with freeze authority', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                6,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });

        it('should create SPL mint with various decimals', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                0, // Zero decimals
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.decimals).toBe(0);
        });
    });

    describe('Token-2022 (TOKEN_2022_PROGRAM_ID)', () => {
        it('should create Token-2022 mint', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                mintKeypair,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            expect(mint.toBase58()).toBe(mintKeypair.publicKey.toBase58());

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );
            expect(fetchedMint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(fetchedMint.isInitialized).toBe(true);
        });

        it('should create Token-2022 mint with freeze authority', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                6,
                mintKeypair,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );
            expect(fetchedMint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });
    });

    describe('decimals variations', () => {
        it('should create mint with 0 decimals', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const { transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                0,
                mintSigner,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const { mint: fetchedMint } = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                CTOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.decimals).toBe(0);
        });

        it('should create SPL mint with max decimals (9)', async () => {
            const mintKeypair = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const { mint, transactionSignature } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const fetchedMint = await getMint(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(fetchedMint.decimals).toBe(9);
        });
    });

    describe('cross-program verification', () => {
        it('should create different mint addresses for different programs', async () => {
            const mintAuthority = Keypair.generate();

            // CToken mint
            const ctokenMintSigner = Keypair.generate();
            const [ctokenMintPda] = findMintAddress(ctokenMintSigner.publicKey);
            const { mint: ctokenMint } = await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                ctokenMintSigner,
            );

            // SPL mint
            const splMintKeypair = Keypair.generate();
            const { mint: splMint } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                splMintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            // Token-2022 mint
            const t22MintKeypair = Keypair.generate();
            const { mint: t22Mint } = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                t22MintKeypair,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            // All mints should be different
            expect(ctokenMint.toBase58()).not.toBe(splMint.toBase58());
            expect(splMint.toBase58()).not.toBe(t22Mint.toBase58());
            expect(ctokenMint.toBase58()).not.toBe(t22Mint.toBase58());

            // CToken mint should be PDA
            expect(ctokenMint.toBase58()).toBe(ctokenMintPda.toBase58());

            // SPL/T22 mints should be keypair pubkeys
            expect(splMint.toBase58()).toBe(
                splMintKeypair.publicKey.toBase58(),
            );
            expect(t22Mint.toBase58()).toBe(
                t22MintKeypair.publicKey.toBase58(),
            );
        });
    });
});
