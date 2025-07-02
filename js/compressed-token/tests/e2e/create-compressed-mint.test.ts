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
    getDefaultAddressTreeInfo,
    buildAndSignTx,
    sendAndConfirmTx,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    createMintInstruction,
    createTokenMetadata,
} from '../../src/mint/instructions';
import { createMint } from '../../src/mint/actions';
import { getMintInterface } from '../../src/mint/helpers';
import { findMintAddress } from '../../src/compressible/derivation';

featureFlags.version = VERSION.V2;

describe('createCompressedMint', () => {
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

    it('should create a compressed mint with metadata and fetch it', async () => {
        const decimals = 9;
        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        const { transactionSignature: signature } = await createMint(
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
            tree: new PublicKey('EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK'),
            queue: new PublicKey(
                'EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK',
            ),
            cpiContext: null,
            treeType: 1,
            nextTreeInfo: null,
        };

        const [mintPda] = findMintAddress(mintSigner2.publicKey);

        await rpc.getValidityProofV2(
            [],
            [
                {
                    address: mintPda.toBytes(),
                    treeInfo: addressTreeInfo,
                },
            ],
        );

        const { transactionSignature: signature } = await createMint(
            rpc,
            payer,
            mintAuthority,
            freezeAuthority.publicKey,
            decimals,
            mintSigner2,
            undefined,
            addressTreeInfo,
            undefined,
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
            tree: new PublicKey('EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK'),
            queue: new PublicKey(
                'EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK',
            ),
            cpiContext: null,
            treeType: 1,
            nextTreeInfo: null,
        };

        const [mintPda] = findMintAddress(mintSigner3.publicKey);

        const validityProof = await rpc.getValidityProofV2(
            [],
            [
                {
                    address: mintPda.toBytes(),
                    treeInfo: addressTreeInfo,
                },
            ],
        );

        const stateTreeInfos = await rpc.getStateTreeInfos();
        const outputStateTreeInfo = stateTreeInfos[0];

        const instruction = createMintInstruction(
            mintSigner3.publicKey,
            decimals,
            mintAuthority.publicKey,
            null,
            payer.publicKey,
            validityProof,
            createTokenMetadata(
                'Some Name',
                'SOME',
                'https://direct.com/metadata.json',
            ),
            addressTreeInfo,
            outputStateTreeInfo,
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
