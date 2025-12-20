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
import { createMintInterface } from '../../../src/v3/actions';
import {
    updateMintAuthority,
    updateFreezeAuthority,
} from '../../../src/v3/actions/update-mint';
import { getMintInterface } from '../../../src/v3/get-mint-interface';
import { findMintAddress } from '../../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('updateMint', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    it('should update mint authority', async () => {
        const mintSigner = Keypair.generate();
        const initialMintAuthority = Keypair.generate();
        const newMintAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            initialMintAuthority,
            null,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const mintInfoBefore = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoBefore.mint.mintAuthority?.toString()).toBe(
            initialMintAuthority.publicKey.toString(),
        );

        const updateSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            newMintAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );
        expect(mintInfoAfter.mint.supply).toBe(0n);
        expect(mintInfoAfter.mint.decimals).toBe(decimals);
    });

    it('should revoke mint authority by setting to null', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            mintAuthority,
            null,
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.mint.mintAuthority).toBe(null);
        expect(mintInfoAfter.mint.supply).toBe(0n);
    });

    it('should update freeze authority', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const initialFreezeAuthority = Keypair.generate();
        const newFreezeAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            initialFreezeAuthority.publicKey,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const mintInfoBefore = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoBefore.mint.freezeAuthority?.toString()).toBe(
            initialFreezeAuthority.publicKey.toString(),
        );

        const updateSig = await updateFreezeAuthority(
            rpc,
            payer,
            mintPda,
            initialFreezeAuthority,
            newFreezeAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.mint.freezeAuthority?.toString()).toBe(
            newFreezeAuthority.publicKey.toString(),
        );
        expect(mintInfoAfter.mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
    });

    it('should revoke freeze authority by setting to null', async () => {
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const freezeAuthority = Keypair.generate();
        const decimals = 6;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            freezeAuthority.publicKey,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateSig = await updateFreezeAuthority(
            rpc,
            payer,
            mintPda,
            freezeAuthority,
            null,
        );
        await rpc.confirmTransaction(updateSig, 'confirmed');

        const mintInfoAfter = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfter.mint.freezeAuthority).toBe(null);
        expect(mintInfoAfter.mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
    });

    it('should update both mint and freeze authorities sequentially', async () => {
        const mintSigner = Keypair.generate();
        const initialMintAuthority = Keypair.generate();
        const initialFreezeAuthority = Keypair.generate();
        const newMintAuthority = Keypair.generate();
        const newFreezeAuthority = Keypair.generate();
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: createSig } = await createMintInterface(
            rpc,
            payer,
            initialMintAuthority,
            initialFreezeAuthority.publicKey,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(createSig, 'confirmed');

        const updateMintAuthSig = await updateMintAuthority(
            rpc,
            payer,
            mintPda,
            initialMintAuthority,
            newMintAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateMintAuthSig, 'confirmed');

        const mintInfoAfterMintAuth = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfterMintAuth.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );

        const updateFreezeAuthSig = await updateFreezeAuthority(
            rpc,
            payer,
            mintPda,
            initialFreezeAuthority,
            newFreezeAuthority.publicKey,
        );
        await rpc.confirmTransaction(updateFreezeAuthSig, 'confirmed');

        const mintInfoAfterBoth = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfoAfterBoth.mint.mintAuthority?.toString()).toBe(
            newMintAuthority.publicKey.toString(),
        );
        expect(mintInfoAfterBoth.mint.freezeAuthority?.toString()).toBe(
            newFreezeAuthority.publicKey.toString(),
        );
    });
});
