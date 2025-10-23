import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    getDefaultAddressTreeInfo,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    createMintInterface,
    updateMintAuthority,
} from '../../src/mint/actions';
import { createTokenMetadata } from '../../src/mint/instructions';
import {
    updateMetadataField,
    updateMetadataAuthority,
    removeMetadataKey,
} from '../../src/mint/actions/update-metadata';
import { getMintInterface } from '../../src/mint/helpers';
import { findMintAddress } from '../../src/compressible/derivation';

featureFlags.version = VERSION.V2;

describe('updateMetadata', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    it('should update metadata name field', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Initial Token',
            'INIT',
            'https://example.com/initial',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const mintInfoBefore = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoBefore.tokenMetadata?.name).toBe('Initial Token');

        const updateSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'name',
            'Updated Token',
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.tokenMetadata?.name).toBe('Updated Token');
        expect(mintInfoAfter.tokenMetadata?.symbol).toBe('INIT');
        expect(mintInfoAfter.tokenMetadata?.uri).toBe(
            'https://example.com/initial',
        );
    });

    it('should update metadata symbol field', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Test Token',
            'TEST',
            'https://example.com/test',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'symbol',
            'UPDATED',
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.tokenMetadata?.symbol).toBe('UPDATED');
        expect(mintInfoAfter.tokenMetadata?.name).toBe('Test Token');
    });

    it('should update metadata uri field', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Token',
            'TKN',
            'https://old.com/metadata',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'uri',
            'https://new.com/metadata',
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.tokenMetadata?.uri).toBe(
            'https://new.com/metadata',
        );
    });

    it('should update metadata authority', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const initialMetadataAuthority = Keypair.generate();
        const newMetadataAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Authority Test',
            'AUTH',
            'https://example.com/auth',
            initialMetadataAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const mintInfoBefore = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoBefore.tokenMetadata?.updateAuthority?.toString()).toBe(
            initialMetadataAuthority.publicKey.toString(),
        );

        const updateSig = await updateMetadataAuthority(
            rpc,
            payer,
            mintPda,
            mintSigner,
            initialMetadataAuthority,
            newMetadataAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.tokenMetadata?.updateAuthority?.toString()).toBe(
            newMetadataAuthority.publicKey.toString(),
        );
    });

    it('should update multiple metadata fields sequentially', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Original Name',
            'ORIG',
            'https://original.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateNameSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'name',
            'New Name',
        );
        await rpc.confirmTransaction(updateNameSig, 'confirmed');

        const mintInfoAfterName = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfterName.tokenMetadata?.name).toBe('New Name');

        const updateSymbolSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'symbol',
            'NEW',
        );
        await rpc.confirmTransaction(updateSymbolSig, 'confirmed');

        const mintInfoAfterSymbol = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfterSymbol.tokenMetadata?.name).toBe('New Name');
        expect(mintInfoAfterSymbol.tokenMetadata?.symbol).toBe('NEW');

        const updateUriSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'uri',
            'https://updated.com',
        );
        await rpc.confirmTransaction(updateUriSig, 'confirmed');

        const mintInfoFinal = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoFinal.tokenMetadata?.name).toBe('New Name');
        expect(mintInfoFinal.tokenMetadata?.symbol).toBe('NEW');
        expect(mintInfoFinal.tokenMetadata?.uri).toBe('https://updated.com');
    });

    it('should fail to update metadata without proper authority', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const wrongAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const initialMetadata = createTokenMetadata(
            'Token',
            'TKN',
            'https://example.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            initialMetadata,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        await expect(
            updateMetadataField(
                rpc,
                payer,
                mintPda,
                mintSigner,
                wrongAuthority,
                'name',
                'Hacked Name',
            ),
        ).rejects.toThrow();
    });

    it('should fail to update mint authority with wrong current authority', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const wrongAuthority = Keypair.generate();
        const newAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
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
        await rpc.confirmTransaction(createSig, 'confirmed');

        await expect(
            updateMintAuthority(
                rpc,
                payer,
                mintPda,
                mintSigner,
                wrongAuthority,
                newAuthority.publicKey,
            ),
        ).rejects.toThrow();
    });

    it('should remove metadata key (idempotent)', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Token with Keys',
            'KEYS',
            'https://keys.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
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
        await rpc.confirmTransaction(createSig, 'confirmed');

        const removeSig = await removeMetadataKey(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'custom_key',
            true,
        );
        await rpc.confirmTransaction(removeSig, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata).toBeDefined();
    });

    it('should update metadata fields with same authority as mint authority', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const metadata = createTokenMetadata(
            'Same Auth Token',
            'SAME',
            'https://same.com',
            mintAuthority.publicKey,
        );

        const { transactionSignature: createSig } = await createMintInterface(
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
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateSig = await updateMetadataField(
            rpc,
            payer,
            mintPda,
            mintSigner,
            mintAuthority,
            'name',
            'Updated by Mint Authority',
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.tokenMetadata?.name).toBe('Updated by Mint Authority');
        expect(mintInfo.tokenMetadata?.updateAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
    });
});
