import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    LIGHT_TOKEN_PROGRAM_ID,
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
import {
    LIGHT_TOKEN_RENT_SPONSOR,
    TOTAL_COMPRESSION_COST,
    COMPRESSIBLE_CTOKEN_RENT_PER_EPOCH,
    DEFAULT_PREPAY_EPOCHS,
    calculateFeePayerCostAtCreation,
} from '../../src/constants';

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

            const address = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                mintPda,
                owner.publicKey,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());

            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);
            expect(accountInfo?.owner.toBase58()).toBe(
                LIGHT_TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should create CToken ATA with explicit LIGHT_TOKEN_PROGRAM_ID', async () => {
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

            const address = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
                false,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                mintPda,
                owner.publicKey,
                false,
                LIGHT_TOKEN_PROGRAM_ID,
            );
            expect(address.toBase58()).toBe(expectedAddress.toBase58());
        });

        it('should use rent sponsor by default (rent sponsor pays rent exemption)', async () => {
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

            // Get rent sponsor balance before ATA creation
            const rentSponsorBalanceBefore = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );

            // Get payer balance before
            const payerBalanceBefore = await rpc.getBalance(payer.publicKey);

            const address = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            // Get balances after
            const rentSponsorBalanceAfter = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );
            const payerBalanceAfter = await rpc.getBalance(payer.publicKey);

            // Verify ATA was created
            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);

            // Rent sponsor should have paid the rent exemption (~890,880 lamports)
            const rentSponsorDiff =
                rentSponsorBalanceBefore - rentSponsorBalanceAfter;
            expect(rentSponsorDiff).toBeGreaterThan(800_000); // ~890,880 for rent exemption

            // Fee payer pays: compression_cost (11K) + 16 epochs rent (~6,400) + tx fees
            // Expected: ~17,400 lamports + tx fees
            const payerDiff = payerBalanceBefore - payerBalanceAfter;
            const expectedFeePayerCost = calculateFeePayerCostAtCreation();
            // Fee payer should pay compression_cost + prepaid rent + tx fees
            expect(payerDiff).toBeGreaterThanOrEqual(expectedFeePayerCost);
            expect(payerDiff).toBeLessThan(expectedFeePayerCost + 20_000); // Allow for tx fees
        });

        it('should correctly fund ATA for 24h with write top-up enforced', async () => {
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

            // Get balances before
            const rentSponsorBalanceBefore = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );
            const payerBalanceBefore = await rpc.getBalance(payer.publicKey);

            const address = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            // Get balances after
            const rentSponsorBalanceAfter = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );
            const payerBalanceAfter = await rpc.getBalance(payer.publicKey);
            const ataBalance = await rpc.getBalance(address);

            // 1) Rent exemption sponsored by tokenRentSponsor
            const rentSponsorDiff =
                rentSponsorBalanceBefore - rentSponsorBalanceAfter;
            expect(rentSponsorDiff).toBeGreaterThan(800_000); // ~890,880 for rent exemption

            // 2) ATA funded for 24h by fee payer
            // Account balance should be: rent_exemption + compression_cost (11K) + 16 epochs rent (~6,400)
            // The fee payer pays the compression_cost + prepaid rent portion
            const payerDiff = payerBalanceBefore - payerBalanceAfter;
            const expectedPrepaidRent =
                DEFAULT_PREPAY_EPOCHS * COMPRESSIBLE_CTOKEN_RENT_PER_EPOCH; // 16 * 400 = 6,400
            const expectedFeePayerCost =
                TOTAL_COMPRESSION_COST + expectedPrepaidRent; // 11,000 + 6,400 = 17,400

            // Payer should pay compression_cost + prepaid rent + tx fees
            expect(payerDiff).toBeGreaterThanOrEqual(expectedFeePayerCost);
            // Allow for tx fees (up to ~20K)
            expect(payerDiff).toBeLessThan(expectedFeePayerCost + 20_000);

            // 3) Verify ATA account balance includes rent exemption + working capital
            // The ATA should have rent_exemption + compression_cost (11K) + prepaid rent
            const accountInfo = await rpc.getAccountInfo(address);
            expect(accountInfo).not.toBe(null);
            const accountDataLength = accountInfo!.data.length;
            const rentExemption =
                await rpc.getMinimumBalanceForRentExemption(accountDataLength);

            // Calculate expected prepaid rent using ACTUAL account size
            const actualRentPerEpoch = 128 + accountDataLength; // base_rent + bytes * 1
            const actualExpectedPrepaidRent =
                DEFAULT_PREPAY_EPOCHS * actualRentPerEpoch;
            const actualExpectedAtaBalance =
                rentExemption +
                TOTAL_COMPRESSION_COST +
                actualExpectedPrepaidRent;

            // ATA balance should EXACTLY match (no tolerance needed when using actual size)
            expect(ataBalance).toBe(actualExpectedAtaBalance);
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

            const addr1 = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            const addr2 = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mintPda,
                owner.publicKey,
            );

            const addr3 = await createAtaInterfaceIdempotent(
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

            const addr1 = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                owner1.publicKey,
            );

            const addr2 = await createAtaInterface(
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

            const address = await createAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

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

            const addr1 = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const addr2 = await createAtaInterfaceIdempotent(
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

            const address = await createAtaInterface(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

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

            const addr1 = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const addr2 = await createAtaInterfaceIdempotent(
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
                LIGHT_TOKEN_PROGRAM_ID,
            );

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
            );

            const address = await createAtaInterface(
                rpc,
                payer,
                mintPda,
                pdaOwner,
                true, // allowOwnerOffCurve
            );

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

            const address = await createAtaInterface(
                rpc,
                payer,
                mint,
                pdaOwner,
                true, // allowOwnerOffCurve
                undefined,
                TOKEN_PROGRAM_ID,
            );

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
            const splAta = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                splMint,
                owner.publicKey,
                false,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const ctokenAta = await createAtaInterfaceIdempotent(
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
            const splAta = await createAtaInterfaceIdempotent(
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
            const t22Ata = await createAtaInterfaceIdempotent(
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
            const ctokenAta = await createAtaInterfaceIdempotent(
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
            const addresses = successful.map(r =>
                (r as PromiseFulfilledResult<PublicKey>).value.toBase58(),
            );
            const uniqueAddresses = [...new Set(addresses)];
            expect(uniqueAddresses.length).toBe(1);
        });
    });
});
