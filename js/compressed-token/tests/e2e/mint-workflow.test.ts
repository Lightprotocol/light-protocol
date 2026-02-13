import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    getDefaultAddressTreeInfo,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import { createMintInterface } from '../../src/v3/actions';
import { createTokenMetadata } from '../../src/v3/instructions';
import {
    updateMintAuthority,
    updateFreezeAuthority,
} from '../../src/v3/actions/update-mint';
import {
    updateMetadataField,
    updateMetadataAuthority,
} from '../../src/v3/actions/update-metadata';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { getAssociatedTokenAddressInterface } from '../../src/';
import { getMintInterface } from '../../src/v3/get-mint-interface';
import { findMintAddress } from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('Complete Mint Workflow', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    it('should execute complete workflow: create mint -> update metadata -> update authorities -> create ATAs', async () => {
        const mintSigner = Keypair.generate();
        const initialMintAuthority = Keypair.generate();
        const initialFreezeAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Workflow Token',
            'WORK',
            'https://workflow.com/initial',
            initialMintAuthority.publicKey,
        );

        const { mint, transactionSignature: createSig } =
            await createMintInterface(
                rpc,
                payer,
                initialMintAuthority,
                initialFreezeAuthority.publicKey,
                decimals,
                mintSigner,
                undefined,
                undefined,
                initialMetadata,
            );
        await rpc.confirmTransaction(createSig, 'confirmed');

        expect(mint.toString()).toBe(mintPda.toString());

        let mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.mintAuthority?.toString()).toBe(
            initialMintAuthority.publicKey.toString(),
        );
        expect(mintInfo.mint.freezeAuthority?.toString()).toBe(
            initialFreezeAuthority.publicKey.toString(),
        );
        expect(mintInfo.tokenMetadata?.name).toBe('Workflow Token');
        expect(mintInfo.tokenMetadata?.symbol).toBe('WORK');

        const updateNameSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            'name',
            'Workflow Token V2',
        );
        await rpc.confirmTransaction(updateNameSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata?.name).toBe('Workflow Token V2');

        const updateUriSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            'uri',
            'https://workflow.com/updated',
        );
        await rpc.confirmTransaction(updateUriSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata?.uri).toBe(
            'https://workflow.com/updated',
        );

        const newMetadataAuthority = Keypair.generate();
        const updateMetadataAuthSig = await updateMetadataAuthority(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            newMetadataAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateMetadataAuthSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata?.updateAuthority?.toString()).toBe(
            newMetadataAuthority.publicKey.toString(),
        );

        const newMintAuthority = Keypair.generate();
        const updateMintAuthSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            newMintAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateMintAuthSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );

        const newFreezeAuthority = Keypair.generate();
        const updateFreezeAuthSig = await updateFreezeAuthority(
            rpc,
            payer,
            mintPda,
            initialFreezeAuthority,
            newFreezeAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateFreezeAuthSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.freezeAuthority?.toString()).toBe(
            newFreezeAuthority.publicKey.toString(),
        );

        const owner1 = Keypair.generate();
        const owner2 = Keypair.generate();
        const owner3 = Keypair.generate();

        const ata1 = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner1.publicKey,
        );

        const ata2 = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner2.publicKey,
        );

        const ata3 = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner3.publicKey,
        );

        const expectedAta1 = getAssociatedTokenAddressInterface(
            mint,
            owner1.publicKey,
        );
        const expectedAta2 = getAssociatedTokenAddressInterface(
            mint,
            owner2.publicKey,
        );
        const expectedAta3 = getAssociatedTokenAddressInterface(
            mint,
            owner3.publicKey,
        );

        expect(ata1.toString()).toBe(expectedAta1.toString());
        expect(ata2.toString()).toBe(expectedAta2.toString());
        expect(ata3.toString()).toBe(expectedAta3.toString());

        const finalMintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(finalMintInfo.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );
        expect(finalMintInfo.mint.freezeAuthority?.toString()).toBe(
            newFreezeAuthority.publicKey.toString(),
        );
        expect(finalMintInfo.tokenMetadata?.updateAuthority?.toString()).toBe(
            newMetadataAuthority.publicKey.toString(),
        );
        expect(finalMintInfo.tokenMetadata?.name).toBe('Workflow Token V2');
        expect(finalMintInfo.tokenMetadata?.uri).toBe(
            'https://workflow.com/updated',
        );
    });

    it('should handle authority revocations in workflow', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const freezeAuthority = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Revoke Test',
            'RVKE',
            'https://revoke.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            freezeAuthority.publicKey,
            decimals,
            mintSigner,
            undefined,
            undefined,
            metadata,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        let mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.freezeAuthority).not.toBe(null);

        const revokeFreezeAuthSig = await updateFreezeAuthority(
            rpc,
            payer,
            mintPda,
            freezeAuthority,
            null,
        );
        await rpc.confirmTransaction(revokeFreezeAuthSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.freezeAuthority).toBe(null);
        expect(mintInfo.mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );

        const owner = Keypair.generate();
        const ataAddress = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mintPda,
            owner.publicKey,
        );

        const expectedAddress = getAssociatedTokenAddressInterface(
            mintPda,
            owner.publicKey,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());

        const accountInfo = await rpc.getAccountInfo(ataAddress);
        expect(accountInfo).not.toBe(null);
    });

    it('should create mint without metadata then create ATAs', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { mint, transactionSignature: createSig } =
            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
            );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata).toBeUndefined();

        const owners = [
            Keypair.generate(),
            Keypair.generate(),
            Keypair.generate(),
        ];

        for (const owner of owners) {
            const ataAddress = await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
            );

            const expectedAddress = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            expect(ataAddress.toString()).toBe(expectedAddress.toString());

            const accountInfo = await rpc.getAccountInfo(ataAddress);
            expect(accountInfo).not.toBe(null);
        }
    });

    it('should update metadata after creating ATAs', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Before ATA',
            'BATA',
            'https://before.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            undefined,
            metadata,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const owner = Keypair.generate();
        const ataAddress = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mintPda,
            owner.publicKey,
        );

        const expectedAddress = getAssociatedTokenAddressInterface(
            mintPda,
            owner.publicKey,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());

        const updateNameSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintAuthority,
            'name',
            'After ATA',
        );
        await rpc.confirmTransaction(updateNameSig, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata?.name).toBe('After ATA');

        const accountInfo = await rpc.getAccountInfo(ataAddress);
        expect(accountInfo).not.toBe(null);
    });

    it('should create mint with all features then verify state consistency', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const freezeAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Full Feature Token',
            'FULL',
            'https://full.com',
            mintAuthority.publicKey,
        );

        const { mint, transactionSignature: createSig } =
            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                freezeAuthority.publicKey,
                decimals,
                mintSigner,
                undefined,
                undefined,
                metadata,
            );
        await rpc.confirmTransaction(createSig, 'confirmed');

        let mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBe(0n);
        expect(mintInfo.mint.decimals).toBe(decimals);
        expect(mintInfo.mint.isInitialized).toBe(true);
        expect(mintInfo.mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
        expect(mintInfo.mint.freezeAuthority?.toString()).toBe(
            freezeAuthority.publicKey.toString(),
        );
        expect(mintInfo.tokenMetadata?.name).toBe('Full Feature Token');
        expect(mintInfo.tokenMetadata?.symbol).toBe('FULL');
        expect(mintInfo.tokenMetadata?.uri).toBe('https://full.com');
        expect(mintInfo.tokenMetadata?.updateAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
        expect(mintInfo.merkleContext).toBeDefined();
        expect(mintInfo.mintContext).toBeDefined();
        expect(mintInfo.mintContext?.version).toBeGreaterThan(0);

        const owner1 = Keypair.generate();
        const owner2 = Keypair.generate();

        const ata1 = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner1.publicKey,
        );

        const ata2 = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner2.publicKey,
        );

        const account1 = await rpc.getAccountInfo(ata1);
        const account2 = await rpc.getAccountInfo(ata2);
        expect(account1).not.toBe(null);
        expect(account2).not.toBe(null);

        const newMintAuthority = Keypair.generate();
        const updateMintAuthSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            mintAuthority,
            newMintAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateMintAuthSig, 'confirmed');

        mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );

        const updateSymbolSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintAuthority,
            'symbol',
            'FULL2',
        );
        await rpc.confirmTransaction(updateSymbolSig, 'confirmed');

        const finalMintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(finalMintInfo.tokenMetadata?.symbol).toBe('FULL2');
        expect(finalMintInfo.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );

        const ata1Again = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner1.publicKey,
        );
        expect(ata1Again.toString()).toBe(ata1.toString());
    });

    it('should create minimal mint then progressively add features and accounts', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { mint, transactionSignature: createSig } =
            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
            );
        await rpc.confirmTransaction(createSig, 'confirmed');

        let mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.freezeAuthority).toBe(null);
        expect(mintInfo.tokenMetadata).toBeUndefined();

        const owner = Keypair.generate();
        const ataAddress = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        const expectedAddress = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        expect(ataAddress.toString()).toBe(expectedAddress.toString());

        const accountInfo = await rpc.getAccountInfo(ataAddress);
        expect(accountInfo).not.toBe(null);
        expect(accountInfo?.data.length).toBeGreaterThan(0);

        const newMintAuthority = Keypair.generate();
        const updateMintAuthSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            mintAuthority,
            newMintAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateMintAuthSig, 'confirmed');

        const finalMintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(finalMintInfo.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );
        expect(finalMintInfo.mint.supply).toBe(0n);
    });

    it('should verify ATA addresses are deterministic', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const owner = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { mint, transactionSignature: createSig } =
            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
            );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const derivedAddressBefore = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        const ataAddress = await createAtaInterfaceIdempotent(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        const derivedAddressAfter = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        expect(ataAddress.toString()).toBe(derivedAddressBefore.toString());
        expect(ataAddress.toString()).toBe(derivedAddressAfter.toString());
        expect(derivedAddressBefore.toString()).toBe(
            derivedAddressAfter.toString(),
        );
    });
});
