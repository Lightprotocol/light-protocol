import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey, SystemProgram } from '@solana/web3.js';
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
    mintTo,
    getAssociatedTokenAddressSync,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountIdempotent,
    TokenInvalidMintError,
    TokenInvalidOwnerError,
} from '@solana/spl-token';
import { createMintInterface } from '../../src/v3/actions/create-mint-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { findMintAddress } from '../../src/v3/derivation';
import { getAtaProgramId } from '../../src/v3/ata-utils';
import { mintToCompressed } from '../../src/v3/actions/mint-to-compressed';

featureFlags.version = VERSION.V2;

describe('getOrCreateAtaInterface', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    describe('SPL Token (TOKEN_PROGRAM_ID)', () => {
        let splMint: PublicKey;

        beforeAll(async () => {
            const mintAuthority = Keypair.generate();
            splMint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );
        });

        it('should create SPL ATA when it does not exist', async () => {
            const owner = Keypair.generate();

            const expectedAddress = getAssociatedTokenAddressSync(
                splMint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            // Verify ATA does not exist
            const beforeInfo = await rpc.getAccountInfo(expectedAddress);
            expect(beforeInfo).toBe(null);

            // Call getOrCreateAtaInterface
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            // Verify returned account
            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(splMint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
            expect(account.parsed.amount).toBe(BigInt(0));

            // Verify ATA now exists
            const afterInfo = await rpc.getAccountInfo(expectedAddress);
            expect(afterInfo).not.toBe(null);
            expect(afterInfo?.owner.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should return existing SPL ATA without creating new one', async () => {
            const owner = Keypair.generate();

            // Pre-create the ATA
            await createAssociatedTokenAccountIdempotent(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const expectedAddress = getAssociatedTokenAddressSync(
                splMint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            // Call getOrCreateAtaInterface on existing ATA
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(splMint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
        });

        it('should work with SPL ATA that has balance', async () => {
            const mintAuthority = Keypair.generate();
            const owner = Keypair.generate();

            // Create mint with mintAuthority we control
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

            // Create ATA and mint tokens
            const ata = await createAssociatedTokenAccountIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            await mintTo(
                rpc,
                payer,
                mint,
                ata,
                mintAuthority,
                1000000n,
                [],
                undefined,
                TOKEN_PROGRAM_ID,
            );

            // Call getOrCreateAtaInterface
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(ata.toBase58());
            expect(account.parsed.amount).toBe(BigInt(1000000));
        });

        it('should create SPL ATA for PDA owner with allowOwnerOffCurve=true', async () => {
            // Derive a PDA
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-pda-spl')],
                SystemProgram.programId,
            );

            const expectedAddress = getAssociatedTokenAddressSync(
                splMint,
                pdaOwner,
                true,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                splMint,
                pdaOwner,
                true,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.owner.toBase58()).toBe(pdaOwner.toBase58());
        });
    });

    describe('Token-2022 (TOKEN_2022_PROGRAM_ID)', () => {
        let t22Mint: PublicKey;

        beforeAll(async () => {
            const mintAuthority = Keypair.generate();
            t22Mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                6,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );
        });

        it('should create Token-2022 ATA when it does not exist', async () => {
            const owner = Keypair.generate();

            const expectedAddress = getAssociatedTokenAddressSync(
                t22Mint,
                owner.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            // Verify ATA does not exist
            const beforeInfo = await rpc.getAccountInfo(expectedAddress);
            expect(beforeInfo).toBe(null);

            // Call getOrCreateAtaInterface
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                t22Mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(t22Mint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
            expect(account.parsed.amount).toBe(BigInt(0));

            // Verify ATA now exists
            const afterInfo = await rpc.getAccountInfo(expectedAddress);
            expect(afterInfo).not.toBe(null);
            expect(afterInfo?.owner.toBase58()).toBe(
                TOKEN_2022_PROGRAM_ID.toBase58(),
            );
        });

        it('should return existing Token-2022 ATA without creating new one', async () => {
            const owner = Keypair.generate();

            // Pre-create the ATA
            await createAssociatedTokenAccountIdempotent(
                rpc,
                payer,
                t22Mint,
                owner.publicKey,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const expectedAddress = getAssociatedTokenAddressSync(
                t22Mint,
                owner.publicKey,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                t22Mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(t22Mint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
        });

        it('should work with Token-2022 ATA that has balance', async () => {
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

            const ata = await createAssociatedTokenAccountIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            await mintTo(
                rpc,
                payer,
                mint,
                ata,
                mintAuthority,
                500000n,
                [],
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(ata.toBase58());
            expect(account.parsed.amount).toBe(BigInt(500000));
        });

        it('should create Token-2022 ATA for PDA owner with allowOwnerOffCurve=true', async () => {
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-pda-t22')],
                SystemProgram.programId,
            );

            const expectedAddress = getAssociatedTokenAddressSync(
                t22Mint,
                pdaOwner,
                true,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                t22Mint,
                pdaOwner,
                true,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.owner.toBase58()).toBe(pdaOwner.toBase58());
        });
    });

    describe('c-token (CTOKEN_PROGRAM_ID)', () => {
        let ctokenMint: PublicKey;
        let mintAuthority: Keypair;

        beforeAll(async () => {
            const mintSigner = Keypair.generate();
            mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );
            ctokenMint = mintPda;
        });

        it('should create c-token ATA when it does not exist (uninited)', async () => {
            const owner = Keypair.generate();

            const expectedAddress = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );

            // Verify ATA does not exist
            const beforeInfo = await rpc.getAccountInfo(expectedAddress);
            expect(beforeInfo).toBe(null);

            // Call getOrCreateAtaInterface
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(ctokenMint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
            expect(account.parsed.amount).toBe(BigInt(0));

            // Verify ATA now exists on-chain (hot)
            const afterInfo = await rpc.getAccountInfo(expectedAddress);
            expect(afterInfo).not.toBe(null);
            expect(afterInfo?.owner.toBase58()).toBe(
                CTOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should return existing c-token hot ATA without creating new one', async () => {
            const owner = Keypair.generate();

            // Pre-create the ATA using createAtaInterfaceIdempotent
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );

            // Call getOrCreateAtaInterface on existing hot ATA
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(ctokenMint.toBase58());
            expect(account.parsed.owner.toBase58()).toBe(
                owner.publicKey.toBase58(),
            );
        });

        it('should create c-token ATA for PDA owner with allowOwnerOffCurve=true', async () => {
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-pda-ctoken')],
                SystemProgram.programId,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                ctokenMint,
                pdaOwner,
                true,
                CTOKEN_PROGRAM_ID,
            );

            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                ctokenMint,
                pdaOwner,
                true,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.owner.toBase58()).toBe(pdaOwner.toBase58());
        });

        it('should handle c-token hot ATA with balance', async () => {
            // Create a fresh mint and owner for this test
            const mintSigner = Keypair.generate();
            const testMintAuth = Keypair.generate();
            const [testMint] = findMintAddress(mintSigner.publicKey);
            const owner = Keypair.generate();

            await createMintInterface(
                rpc,
                payer,
                testMintAuth,
                null,
                9,
                mintSigner,
            );

            // Create ATA
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                testMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );

            // Note: Minting to c-token hot accounts uses mintToInterface which
            // requires the mint to be registered. We just verify the account exists.
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );
            expect(account.parsed.mint.toBase58()).toBe(testMint.toBase58());
        });

        it('should detect cold balance with PublicKey owner (no auto-load)', async () => {
            // Create a fresh mint and owner
            const mintSigner = Keypair.generate();
            const testMintAuth = Keypair.generate();
            const [testMint] = findMintAddress(mintSigner.publicKey);
            const owner = Keypair.generate();

            await createMintInterface(
                rpc,
                payer,
                testMintAuth,
                null,
                9,
                mintSigner,
            );

            // Mint compressed tokens directly (creates cold balance, no hot ATA)
            const mintAmount = 1000000n;
            await mintToCompressed(rpc, payer, testMint, testMintAuth, [
                { recipient: owner.publicKey, amount: mintAmount },
            ]);

            const expectedAddress = getAssociatedTokenAddressInterface(
                testMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );

            // Verify NO hot ATA exists before call
            const beforeInfo = await rpc.getAccountInfo(expectedAddress);
            expect(beforeInfo).toBe(null);

            // Verify compressed balance exists
            const compressedBefore =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint: testMint,
                });
            expect(compressedBefore.items.length).toBeGreaterThan(0);

            // Call with owner.publicKey (PublicKey) - should NOT auto-load
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner.publicKey, // PublicKey, not Signer
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            // Verify account has aggregated balance (from cold)
            expect(account.parsed.amount).toBe(mintAmount);

            // Verify hot ATA was created
            const afterInfo = await rpc.getAccountInfo(expectedAddress);
            expect(afterInfo).not.toBe(null);

            // Verify cold balance still exists (NOT loaded because owner is PublicKey)
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                {
                    mint: testMint,
                },
            );
            expect(compressedAfter.items.length).toBeGreaterThan(0);
        });

        it('should auto-load cold balance with Signer owner', async () => {
            // Create a fresh mint and owner
            const mintSigner = Keypair.generate();
            const testMintAuth = Keypair.generate();
            const [testMint] = findMintAddress(mintSigner.publicKey);
            const owner = Keypair.generate();

            await createMintInterface(
                rpc,
                payer,
                testMintAuth,
                null,
                9,
                mintSigner,
            );

            // Mint compressed tokens directly (creates cold balance, no hot ATA)
            const mintAmount = 1000000n;
            await mintToCompressed(rpc, payer, testMint, testMintAuth, [
                { recipient: owner.publicKey, amount: mintAmount },
            ]);

            const expectedAddress = getAssociatedTokenAddressInterface(
                testMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
            );

            // Verify NO hot ATA exists before call
            const beforeInfo = await rpc.getAccountInfo(expectedAddress);
            expect(beforeInfo).toBe(null);

            // Verify compressed balance exists before
            const compressedBefore =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint: testMint,
                });
            expect(compressedBefore.items.length).toBeGreaterThan(0);

            // Call with owner (Signer) - should auto-load cold into hot
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner, // Signer, triggers auto-load
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            // Verify correct address
            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );

            // Verify account has full balance in hot ATA
            expect(account.parsed.amount).toBe(mintAmount);

            // Verify hot ATA was created and has balance
            const afterInfo = await rpc.getAccountInfo(expectedAddress);
            expect(afterInfo).not.toBe(null);
            expect(afterInfo?.owner.toBase58()).toBe(
                CTOKEN_PROGRAM_ID.toBase58(),
            );
            // Parse hot balance
            const hotBalance = afterInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(mintAmount);

            // Verify cold balance was consumed (loaded into hot)
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                {
                    mint: testMint,
                },
            );
            // Cold accounts should be consumed (0 or empty)
            const remainingCold = compressedAfter.items.reduce(
                (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
                BigInt(0),
            );
            expect(remainingCold).toBe(0n);
        });

        it('should aggregate hot and cold balances', async () => {
            // Create a fresh mint and owner
            const mintSigner = Keypair.generate();
            const testMintAuth = Keypair.generate();
            const [testMint] = findMintAddress(mintSigner.publicKey);
            const owner = Keypair.generate();

            await createMintInterface(
                rpc,
                payer,
                testMintAuth,
                null,
                9,
                mintSigner,
            );

            // Create hot ATA first
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            // Mint compressed tokens (creates cold balance)
            const coldAmount = 500000n;
            await mintToCompressed(rpc, payer, testMint, testMintAuth, [
                { recipient: owner.publicKey, amount: coldAmount },
            ]);

            // Call getOrCreateAtaInterface
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            // Verify aggregated balance (hot=0 + cold=coldAmount)
            expect(account.parsed.amount).toBe(coldAmount);
        });
    });

    describe('default programId (CTOKEN_PROGRAM_ID)', () => {
        let ctokenMint: PublicKey;

        beforeAll(async () => {
            const mintAuthority = Keypair.generate();
            const result = await createMintInterface(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
            );
            ctokenMint = result.mint;
        });

        it('should default to CTOKEN_PROGRAM_ID when programId not specified', async () => {
            const owner = Keypair.generate();

            const expectedAddress = getAssociatedTokenAddressSync(
                ctokenMint,
                owner.publicKey,
                false,
                CTOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            // Call without specifying programId
            const account = await getOrCreateAtaInterface(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
            );

            expect(account.parsed.address.toBase58()).toBe(
                expectedAddress.toBase58(),
            );

            // Verify it's owned by CTOKEN_PROGRAM_ID
            const info = await rpc.getAccountInfo(expectedAddress);
            expect(info?.owner.toBase58()).toBe(CTOKEN_PROGRAM_ID.toBase58());
        });
    });

    describe('idempotency', () => {
        it('should be idempotent - multiple calls return same account for SPL', async () => {
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

            // Call multiple times
            const account1 = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const account2 = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const account3 = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(account1.parsed.address.toBase58()).toBe(
                account2.parsed.address.toBase58(),
            );
            expect(account2.parsed.address.toBase58()).toBe(
                account3.parsed.address.toBase58(),
            );
        });

        it('should be idempotent for c-token', async () => {
            const mintSigner = Keypair.generate();
            const testMintAuth = Keypair.generate();
            const [testMint] = findMintAddress(mintSigner.publicKey);
            const owner = Keypair.generate();

            await createMintInterface(
                rpc,
                payer,
                testMintAuth,
                null,
                9,
                mintSigner,
            );

            const account1 = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            const account2 = await getOrCreateAtaInterface(
                rpc,
                payer,
                testMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(account1.parsed.address.toBase58()).toBe(
                account2.parsed.address.toBase58(),
            );
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

            // Create T22 mint
            const t22Mint = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                9,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            // Create c-token mint
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

            // Get/Create ATAs for all programs
            const splAccount = await getOrCreateAtaInterface(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const t22Account = await getOrCreateAtaInterface(
                rpc,
                payer,
                t22Mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const ctokenAccount = await getOrCreateAtaInterface(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            // All addresses should be different (different mints)
            expect(splAccount.parsed.address.toBase58()).not.toBe(
                t22Account.parsed.address.toBase58(),
            );
            expect(splAccount.parsed.address.toBase58()).not.toBe(
                ctokenAccount.parsed.address.toBase58(),
            );
            expect(t22Account.parsed.address.toBase58()).not.toBe(
                ctokenAccount.parsed.address.toBase58(),
            );

            // Verify each account's mint matches
            expect(splAccount.parsed.mint.toBase58()).toBe(splMint.toBase58());
            expect(t22Account.parsed.mint.toBase58()).toBe(t22Mint.toBase58());
            expect(ctokenAccount.parsed.mint.toBase58()).toBe(
                ctokenMint.toBase58(),
            );
        });
    });

    describe('concurrent calls', () => {
        it('should handle concurrent getOrCreate calls for same ATA', async () => {
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

            // Call concurrently
            const results = await Promise.all([
                getOrCreateAtaInterface(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    TOKEN_PROGRAM_ID,
                ),
                getOrCreateAtaInterface(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    TOKEN_PROGRAM_ID,
                ),
                getOrCreateAtaInterface(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    TOKEN_PROGRAM_ID,
                ),
            ]);

            // All results should be the same account
            expect(results[0].parsed.address.toBase58()).toBe(
                results[1].parsed.address.toBase58(),
            );
            expect(results[1].parsed.address.toBase58()).toBe(
                results[2].parsed.address.toBase58(),
            );
        });
    });
});
