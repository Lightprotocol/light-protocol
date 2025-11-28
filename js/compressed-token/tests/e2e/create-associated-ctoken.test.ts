import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    getDefaultAddressTreeInfo,
} from '@lightprotocol/stateless.js';
import { createMintInterface } from '../../src/mint/actions';
import {
    createAssociatedCTokenAccount,
    createAssociatedCTokenAccountIdempotent,
} from '../../src/mint/actions/create-associated-ctoken';
import { createTokenMetadata } from '../../src/mint/instructions';
import { getAssociatedCTokenAddress } from '../../src/compressible';
import { findMintAddress } from '../../src/compressible/derivation';

featureFlags.version = VERSION.V2;

describe('createAssociatedCTokenAccount', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    it('should create an associated ctoken account', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { address: ataAddress, transactionSignature: createAtaSig } =
            await createAssociatedCTokenAccount(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );
        await rpc.confirmTransaction(createAtaSig, 'confirmed');

        const expectedAddress = getAssociatedCTokenAddress(
            owner.publicKey,
            mintPda,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());

        const accountInfo = await rpc.getAccountInfo(ataAddress);
        expect(accountInfo).not.toBe(null);
        expect(accountInfo?.owner.toString()).toBe(
            'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
        );
    });

    it('should fail to create associated ctoken account twice (non-idempotent)', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { transactionSignature: createAtaSig } =
            await createAssociatedCTokenAccount(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );
        await rpc.confirmTransaction(createAtaSig, 'confirmed');

        await expect(
            createAssociatedCTokenAccount(rpc, payer, owner.publicKey, mintPda),
        ).rejects.toThrow();
    });

    it('should create associated ctoken account idempotently', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { address: ataAddress1, transactionSignature: createAtaSig1 } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );
        await rpc.confirmTransaction(createAtaSig1, 'confirmed');

        const expectedAddress = getAssociatedCTokenAddress(
            owner.publicKey,
            mintPda,
        );
        expect(ataAddress1.toString()).toBe(expectedAddress.toString());

        const { address: ataAddress2, transactionSignature: createAtaSig2 } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );
        await rpc.confirmTransaction(createAtaSig2, 'confirmed');

        expect(ataAddress2.toString()).toBe(ataAddress1.toString());

        const accountInfo = await rpc.getAccountInfo(ataAddress2);
        expect(accountInfo).not.toBe(null);
    });

    it('should create associated accounts for multiple owners for same mint', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner1 = Keypair.generate();
        const owner2 = Keypair.generate();
        const owner3 = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { address: ata1 } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner1.publicKey,
            mintPda,
        );

        const { address: ata2 } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner2.publicKey,
            mintPda,
        );

        const { address: ata3 } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner3.publicKey,
            mintPda,
        );

        expect(ata1.toString()).not.toBe(ata2.toString());
        expect(ata1.toString()).not.toBe(ata3.toString());
        expect(ata2.toString()).not.toBe(ata3.toString());

        const expectedAta1 = getAssociatedCTokenAddress(
            owner1.publicKey,
            mintPda,
        );
        const expectedAta2 = getAssociatedCTokenAddress(
            owner2.publicKey,
            mintPda,
        );
        const expectedAta3 = getAssociatedCTokenAddress(
            owner3.publicKey,
            mintPda,
        );

        expect(ata1.toString()).toBe(expectedAta1.toString());
        expect(ata2.toString()).toBe(expectedAta2.toString());
        expect(ata3.toString()).toBe(expectedAta3.toString());
    });

    it('should handle idempotent creation with concurrent calls', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const createPromises = Array(3)
            .fill(null)
            .map(() =>
                createAssociatedCTokenAccountIdempotent(
                    rpc,
                    payer,
                    owner.publicKey,
                    mintPda,
                ),
            );

        const results = await Promise.allSettled(createPromises);

        const successfulResults = results.filter(r => r.status === 'fulfilled');
        expect(successfulResults.length).toBeGreaterThan(0);

        if (
            successfulResults.length > 0 &&
            successfulResults[0].status === 'fulfilled'
        ) {
            const expectedAddress = getAssociatedCTokenAddress(
                owner.publicKey,
                mintPda,
            );
            expect(successfulResults[0].value.address.toString()).toBe(
                expectedAddress.toString(),
            );
        }
    });
});

describe('createMint -> createAssociatedCTokenAccount flow', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    it('should create mint then create multiple associated accounts', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Flow Test Token',
            'FLOW',
            'https://flow.com/metadata',
            mintAuthority.publicKey,
        );

        const { mint, transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            metadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        expect(mint.toString()).toBe(mintPda.toString());

        const owner1 = Keypair.generate();
        const owner2 = Keypair.generate();

        const { address: ata1, transactionSignature: createAta1Sig } =
            await createAssociatedCTokenAccount(
                rpc,
                payer,
                owner1.publicKey,
                mint,
            );
        await rpc.confirmTransaction(createAta1Sig, 'confirmed');

        const { address: ata2, transactionSignature: createAta2Sig } =
            await createAssociatedCTokenAccount(
                rpc,
                payer,
                owner2.publicKey,
                mint,
            );
        await rpc.confirmTransaction(createAta2Sig, 'confirmed');

        const expectedAta1 = getAssociatedCTokenAddress(owner1.publicKey, mint);
        const expectedAta2 = getAssociatedCTokenAddress(owner2.publicKey, mint);

        expect(ata1.toString()).toBe(expectedAta1.toString());
        expect(ata2.toString()).toBe(expectedAta2.toString());

        const account1Info = await rpc.getAccountInfo(ata1);
        const account2Info = await rpc.getAccountInfo(ata2);

        expect(account1Info).not.toBe(null);
        expect(account2Info).not.toBe(null);
        expect(account1Info?.owner.toString()).toBe(
            'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
        );
        expect(account2Info?.owner.toString()).toBe(
            'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
        );
    });

    it('should create mint with freeze authority then create associated account', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const freezeAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { mint, transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            freezeAuthority.publicKey,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { address: ataAddress, transactionSignature: createAtaSig } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mint,
            );
        await rpc.confirmTransaction(createAtaSig, 'confirmed');

        const expectedAddress = getAssociatedCTokenAddress(
            owner.publicKey,
            mint,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());
    });

    it('should verify different mints produce different ATAs for same owner', async () => {
        const owner = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();

        const mintSigner1 = Keypair.generate();
        const mintAuthority1 = Keypair.generate();
        const [mintPda1] = findMintAddress(mintSigner1.publicKey);

        const { transactionSignature: createMint1Sig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority1,
            null,
            decimals,
            mintSigner1,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMint1Sig, 'confirmed');

        const mintSigner2 = Keypair.generate();
        const mintAuthority2 = Keypair.generate();
        const [mintPda2] = findMintAddress(mintSigner2.publicKey);

        const { transactionSignature: createMint2Sig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority2,
            null,
            decimals,
            mintSigner2,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMint2Sig, 'confirmed');

        const { address: ata1 } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner.publicKey,
            mintPda1,
        );

        const { address: ata2 } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner.publicKey,
            mintPda2,
        );

        expect(ata1.toString()).not.toBe(ata2.toString());

        const expectedAta1 = getAssociatedCTokenAddress(
            owner.publicKey,
            mintPda1,
        );
        const expectedAta2 = getAssociatedCTokenAddress(
            owner.publicKey,
            mintPda2,
        );

        expect(ata1.toString()).toBe(expectedAta1.toString());
        expect(ata2.toString()).toBe(expectedAta2.toString());
    });

    it('should work with pre-existing mint (not created in same test)', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        await new Promise(resolve => setTimeout(resolve, 1000));

        const owner = Keypair.generate();
        const { address: ataAddress } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            owner.publicKey,
            mintPda,
        );

        const expectedAddress = getAssociatedCTokenAddress(
            owner.publicKey,
            mintPda,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());
    });

    it('should verify idempotent behavior with explicit multiple calls', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createMintSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createMintSig, 'confirmed');

        const { address: ataAddress1 } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );

        const { address: ataAddress2 } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );

        const { address: ataAddress3 } =
            await createAssociatedCTokenAccountIdempotent(
                rpc,
                payer,
                owner.publicKey,
                mintPda,
            );

        expect(ataAddress1.toString()).toBe(ataAddress2.toString());
        expect(ataAddress2.toString()).toBe(ataAddress3.toString());
    });

    it('should match SPL-style ATA derivation pattern', async () => {
        const owner = PublicKey.unique();
        const mint = PublicKey.unique();

        const ataAddress = getAssociatedCTokenAddress(owner, mint);

        const [expectedAddress, bump] = PublicKey.findProgramAddressSync(
            [
                owner.toBuffer(),
                new PublicKey(
                    'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
                ).toBuffer(),
                mint.toBuffer(),
            ],
            new PublicKey('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
        );

        expect(ataAddress.toString()).toBe(expectedAddress.toString());
        expect(bump).toBeGreaterThanOrEqual(0);
        expect(bump).toBeLessThanOrEqual(255);
    });
});
