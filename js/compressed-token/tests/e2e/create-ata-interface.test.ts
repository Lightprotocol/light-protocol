import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
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
    createMint,
    getMint,
    getAssociatedTokenAddressSync,
    ASSOCIATED_TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { createMintInterface } from '../../src/v3/actions';
import {
    createAtaInterface,
    createAtaInterfaceIdempotent,
} from '../../src/v3/actions/create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { findMintAddress } from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('createAtaInterface', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    describe('CToken (default programId)', () => {
        it('should create CToken ATA with default programId', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressInterface(
                mintPda,
                owner.publicKey,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());

            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);
            expect(accountInfo?.owner.toBase58()).toBe(
                CTOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should create CToken ATA with explicit CTOKEN_PROGRAM_ID', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                6,
                mintSigner,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
                false,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressInterface(
                mintPda,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());
        });

        it('should fail creating CToken ATA twice (non-idempotent)', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            await createAtaInterface(rpc, payer, mintPda, owner.publicKey);

            await expect(
                createAtaInterface(rpc, payer, mintPda, owner.publicKey),
            ).rejects.toThrow();
        });

        it('should create CToken ATA idempotently', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const { address: addr1 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            const { address: addr2 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            const { address: addr3 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            expect(addr1.toBase58()).toBe(addr2.toBase58());
            expect(addr2.toBase58()).toBe(addr3.toBase58());
        });

        it('should create CToken ATAs for multiple owners', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner1 = Keypair.generate();
            const owner2 = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const { address: addr1 } = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner1.publicKey,
            );

            const { address: addr2 } = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner2.publicKey,
            );

            expect(addr1.toBase58()).not.toBe(addr2.toBase58());

            const expected1 = getAssociatedTokenAddressInterface(
                mintPda,
                owner1.publicKey,
            );
            const expected2 = getAssociatedTokenAddressInterface(
                mintPda,
                owner2.publicKey,
            );

            expect(addr1.toBase58()).toBe(expected1.toBase58());
            expect(addr2.toBase58()).toBe(expected2.toBase58());
        });
    });

    describe('SPL Token (TOKEN_PROGRAM_ID)', () => {
        it('should create SPL Token ATA', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressSync(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());

            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);
            expect(accountInfo?.owner.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should create SPL Token ATA idempotently', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                6,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const { address: addr1 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const { address: addr2 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(addr1.toBase58()).toBe(addr2.toBase58());
        });

        it('should fail creating SPL Token ATA twice (non-idempotent)', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await createAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await expect(
                createAtaInterface(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    false,
                    undefined,
                    TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow();
        });
    });

    describe('Token-2022 (TOKEN_2022_PROGRAM_ID)', () => {
        it('should create Token-2022 ATA', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressSync(
                mint,
                owner.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());

            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);
            expect(accountInfo?.owner.toBase58()).toBe(
                TOKEN_2022_PROGRAM_ID.toBase58(),
            );
        });

        it('should create Token-2022 ATA idempotently', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                6,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const { address: addr1 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const { address: addr2 } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(addr1.toBase58()).toBe(addr2.toBase58());
        });
    });

    describe('PDA owner (allowOwnerOffCurve)', () => {
        it('should create CToken ATA for PDA owner with allowOwnerOffCurve=true', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            // Create a PDA owner
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-pda-owner')],
                CTOKEN_PROGRAM_ID,
            );

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                pdaOwner,
                true, // allowOwnerOffCurve
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressInterface(
                mintPda,
                pdaOwner,
                true,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());
        });

        it('should create SPL Token ATA for PDA owner with allowOwnerOffCurve=true', async () => {
            const mintAuthority = Keypair.generate();

            // Create a PDA owner
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-spl-pda-owner')],
                TOKEN_PROGRAM_ID,
            );

            const mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const { address, transactionSignature } = await createAtaInterface(
                rpc,
                payer,
                mint,
                pdaOwner,
                true, // allowOwnerOffCurve
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await rpc.confirmTransaction(transactionSignature, 'confirmed');

            const expectedAddress = getAssociatedTokenAddressSync(
                mint,
                pdaOwner,
                true,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());
        });
    });

    describe('cross-program verification', () => {
        it('should produce different ATAs for same owner/mint with different programs', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            // Create SPL mint
            const splMint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            // Create CToken mint
            const mintSigner = Keypair.generate();
            const [ctokenMint] = findMintAddress(mintSigner.publicKey);
            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            // Create ATAs for both
            const { address: splAta } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const { address: ctokenAta } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
            );

            // ATAs should be different (different mints and programs)
            expect(splAta.toBase58()).not.toBe(ctokenAta.toBase58());
        });

        it('should match expected derivation for each program', async () => {
            const owner = Keypair.generate();

            // SPL Token
            const splMintAuth = Keypair.generate();
            const splMint = await createMint(
                rpc,
                payer,
                splMintAuth.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            const { address: splAta } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            const expectedSplAta = getAssociatedTokenAddressSync(
                splMint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(splAta.toBase58()).toBe(expectedSplAta.toBase58());

            // Token-2022
            const t22MintAuth = Keypair.generate();
            const t22Mint = await createMint(
                rpc,
                payer,
                t22MintAuth.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );
            const { address: t22Ata } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                t22Mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );
            const expectedT22Ata = getAssociatedTokenAddressSync(
                t22Mint,
                owner.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(t22Ata.toBase58()).toBe(expectedT22Ata.toBase58());

            // CToken
            const mintSigner = Keypair.generate();
            const ctokenMintAuth = Keypair.generate();
            const [ctokenMint] = findMintAddress(mintSigner.publicKey);
            await createMintInterface(
                rpc,
                payer,
                ctokenMintAuth,
                null,
                9,
                mintSigner,
            );
            const { address: ctokenAta } = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
            );
            const expectedCtokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            expect(ctokenAta.toBase58()).toBe(expectedCtokenAta.toBase58());
        });
    });

    describe('concurrent calls', () => {
        it('should handle concurrent idempotent calls for CToken', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const promises = Array(3)
                .fill(null)
                .map(() =>
                    createAtaInterfaceIdempotent(
                        rpc,
                        payer,
                        mintPda,
                        owner.publicKey,
                    ),
                );

            const results = await Promise.allSettled(promises);
            const successful = results.filter(r => r.status === 'fulfilled');

            expect(successful.length).toBeGreaterThan(0);

            // All successful results should have same address
            const addresses = successful.map(
                r => (r as PromiseFulfilledResult<any>).value.address.toBase58(),
            );
            const uniqueAddresses = [...new Set(addresses)];
            expect(uniqueAddresses.length).toBe(1);
        });
    });
});
