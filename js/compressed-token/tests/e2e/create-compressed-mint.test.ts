import { describe, it, expect, beforeAll } from 'vitest';
import {
    PublicKey,
    Keypair,
    Signer,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    buildAndSignTx,
    sendAndConfirmTx,
    CTOKEN_PROGRAM_ID,
    DerivationMode,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    createMintInstruction,
    createTokenMetadata,
} from '../../src/v3/instructions';
import { createMintInterface } from '../../src/v3/actions';
import { getMintInterface } from '../../src/v3/get-mint-interface';
import { findMintAddress } from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('createMintInterface (compressed)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintSigner: Keypair;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintSigner = Keypair.generate();
        mintAuthority = Keypair.generate();
    });

    it('should create a compressed mint and fetch it', async () => {
        const decimals = 9;
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: signature } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            { skipPreflight: true },
        );

        await rpc.confirmTransaction(signature, 'confirmed');
        const { mint, merkleContext, mintContext } = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );

        expect(mint.address.toString()).toBe(mintPda.toString());
        expect(mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
        expect(mint.supply).toBe(0n);
        expect(mint.isInitialized).toBe(true);
        expect(mint.freezeAuthority).toBe(null);
        expect(merkleContext).toBeDefined();
        expect(mintContext).toBeDefined();
    });

    it('should create a compressed mint with freeze authority', async () => {
        const decimals = 6;
        const freezeAuthority = Keypair.generate();
        const mintSigner2 = Keypair.generate();

        const addressTreeInfo = {
            tree: new PublicKey('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
            queue: new PublicKey('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
            cpiContext: undefined,
            treeType: 1,
            nextTreeInfo: null,
        };

        const [mintPda] = findMintAddress(mintSigner2.publicKey);

        await rpc.getValidityProofV2(
            [],
            [
                {
                    address: Uint8Array.from(mintPda.toBytes()),
                    treeInfo: addressTreeInfo,
                },
            ],
            DerivationMode.compressible,
        );

        const { transactionSignature: signature } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            freezeAuthority.publicKey,
            decimals,
            mintSigner2,
        );

        await rpc.confirmTransaction(signature, 'confirmed');
        const { mint, merkleContext, mintContext } = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );

        expect(mint.address.toString()).toBe(mintPda.toString());
        expect(mint.mintAuthority?.toString()).toBe(
            mintAuthority.publicKey.toString(),
        );
        expect(mint.freezeAuthority?.toString()).toBe(
            freezeAuthority.publicKey.toString(),
        );
        expect(mint.isInitialized).toBe(true);
        expect(merkleContext).toBeDefined();
        expect(mintContext).toBeDefined();
    });

    it('should create compressed mint using instruction builder directly', async () => {
        const decimals = 2;
        const mintSigner3 = Keypair.generate();

        const addressTreeInfo = {
            tree: new PublicKey('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
            queue: new PublicKey('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx'),
            cpiContext: undefined,
            treeType: 1,
            nextTreeInfo: null,
        };

        const [mintPda] = findMintAddress(mintSigner3.publicKey);

        const validityProof = await rpc.getValidityProofV2(
            [],
            [
                {
                    address: Uint8Array.from(mintPda.toBytes()),
                    treeInfo: addressTreeInfo,
                },
            ],
            DerivationMode.compressible,
        );

        const outputStateTreeInfo = selectStateTreeInfo(
            await rpc.getStateTreeInfos(),
        );

        const instruction = createMintInstruction(
            mintSigner3.publicKey,
            decimals,
            mintAuthority.publicKey,
            null,
            payer.publicKey,
            validityProof,
            addressTreeInfo,
            outputStateTreeInfo,
            createTokenMetadata(
                'Some Name',
                'SOME',
                'https://direct.com/metadata.json',
            ),
        );

        const { blockhash } = await rpc.getLatestBlockhash();

        const transaction = buildAndSignTx(
            [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }),
                instruction,
            ],
            payer,
            blockhash,
            [mintSigner3, mintAuthority],
        );

        await sendAndConfirmTx(rpc, transaction);

        const { mint } = await getMintInterface(
            rpc,
            mintPda,
            undefined,
            CTOKEN_PROGRAM_ID,
        );

        expect(mint.isInitialized).toBe(true);
        expect(mint.address.toString()).toBe(mintPda.toString());
    });
});
