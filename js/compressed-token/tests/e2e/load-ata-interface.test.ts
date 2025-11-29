import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    CTOKEN_PROGRAM_ID,
    getDefaultAddressTreeInfo,
    createRpc,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, decompress } from '../../src/actions';
import {
    createAssociatedTokenAccount,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { loadAtaInterfaceInstructions } from '../../src/mint/actions/load-ata-interface';
import { getAtaAddressInterface } from '../../src/mint/actions/create-ata-interface';
import { getAtaProgramId } from '../../src/utils';
import { createMintInterface } from '../../src/mint/actions/create-mint-interface';
import { mintToCompressed } from '../../src/mint/actions/mint-to-compressed';
import { findMintAddress } from '../../src/compressible/derivation';

// Force V2 for CToken tests
featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('loadAtaInterface with SPL mint', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        // Create SPL mint with token pool
        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    describe('getAtaAddressInterface helper', () => {
        it('should derive correct CToken ATA address', () => {
            const owner = Keypair.generate().publicKey;
            const ctokenAta = getAtaAddressInterface(mint, owner);

            // Verify it matches the expected derivation
            const expectedAta = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                CTOKEN_PROGRAM_ID,
                getAtaProgramId(CTOKEN_PROGRAM_ID),
            );

            expect(ctokenAta.toString()).toBe(expectedAta.toString());
        });

        it('should derive different addresses for different owners', () => {
            const owner1 = Keypair.generate().publicKey;
            const owner2 = Keypair.generate().publicKey;

            const ata1 = getAtaAddressInterface(mint, owner1);
            const ata2 = getAtaAddressInterface(mint, owner2);

            expect(ata1.toString()).not.toBe(ata2.toString());
        });

        it('should derive different addresses for different mints', () => {
            const owner = Keypair.generate().publicKey;
            const mint2 = Keypair.generate().publicKey;

            const ata1 = getAtaAddressInterface(mint, owner);
            const ata2 = getAtaAddressInterface(mint2, owner);

            expect(ata1.toString()).not.toBe(ata2.toString());
        });
    });

    describe('loadAtaInterfaceInstructions with SPL mint', () => {
        it('should return empty sources when no tokens exist', async () => {
            const owner = Keypair.generate();

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { tokenPoolInfos },
            );

            expect(result.ctokenAta).toBeDefined();
            expect(result.sources.length).toBe(0);
            expect(result.totalAmount).toBe(BigInt(0));
            expect(result.requiresProof).toBe(false);
        });

        it('should detect SPL tokens as source', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Create SPL ATA and add tokens
            const splAta = await createAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                owner.publicKey,
            );

            // Mint compressed tokens first, then decompress to SPL ATA
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
            await decompress(
                rpc,
                payer,
                mint,
                bn(500),
                owner,
                splAta,
                selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)),
            );

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { tokenPoolInfos },
            );

            expect(result.sources.length).toBeGreaterThan(0);

            // Should detect SPL source
            const splSource = result.sources.find(s => s.type === 'spl');
            expect(splSource).toBeDefined();
            expect(splSource!.amount).toBe(BigInt(500));

            // Should also detect remaining compressed tokens
            const compressedSource = result.sources.find(
                s => s.type === 'compressed',
            );
            expect(compressedSource).toBeDefined();
            expect(compressedSource!.amount).toBe(BigInt(500));

            expect(result.totalAmount).toBe(BigInt(1000));
            expect(result.requiresProof).toBe(true);
        });

        it('should detect only compressed tokens as source', async () => {
            const owner = Keypair.generate();

            // Mint compressed tokens only
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(750),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { tokenPoolInfos },
            );

            expect(result.sources.length).toBe(1);

            const compressedSource = result.sources.find(
                s => s.type === 'compressed',
            );
            expect(compressedSource).toBeDefined();
            expect(compressedSource!.amount).toBe(BigInt(750));

            expect(result.totalAmount).toBe(BigInt(750));
            expect(result.requiresProof).toBe(true);
            expect(result.compressedAccounts).toBeDefined();
            expect(result.compressedAccounts!.length).toBeGreaterThan(0);
        });

        it('should handle zero-balance SPL ATA (not treated as source)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Create SPL ATA but don't fund it
            await createAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                owner.publicKey,
            );

            // Mint some compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { tokenPoolInfos },
            );

            // Should only have compressed source, not SPL (since balance is 0)
            expect(result.sources.length).toBe(1);
            expect(result.sources[0].type).toBe('compressed');
            expect(result.totalAmount).toBe(BigInt(100));
        });

        it('should correctly derive CToken ATA address', async () => {
            const owner = Keypair.generate();

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { tokenPoolInfos },
            );

            const expectedCtokenAta = getAtaAddressInterface(
                mint,
                owner.publicKey,
            );
            expect(result.ctokenAta.toString()).toBe(
                expectedCtokenAta.toString(),
            );
        });

        it('should work with provided mintProgramId', async () => {
            const owner = Keypair.generate();

            // Mint some tokens first
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(50),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                {
                    mintProgramId: TOKEN_PROGRAM_ID, // Explicitly provide
                    tokenPoolInfos,
                },
            );

            expect(result.ctokenAta).toBeDefined();
            expect(result.sources.length).toBe(1);
        });
    });
});

describe('loadAtaInterface with CToken mint', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintSigner: Keypair;
    let mintAuthority: Keypair;
    let mint: PublicKey;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintSigner = Keypair.generate();
        mintAuthority = Keypair.generate();

        const addressTreeInfo = getDefaultAddressTreeInfo();
        const [mintPda] = findMintAddress(mintSigner.publicKey);

        // Create CToken mint
        const { transactionSignature } = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            TEST_TOKEN_DECIMALS,
            mintSigner,
            undefined,
            addressTreeInfo,
            undefined,
        );
        await rpc.confirmTransaction(transactionSignature, 'confirmed');

        mint = mintPda;
    }, 60_000);

    describe('loadAtaInterfaceInstructions with CToken mint', () => {
        it('should return empty sources when no tokens exist', async () => {
            const owner = Keypair.generate();

            // For CToken mints, pass CTOKEN_PROGRAM_ID since there's no on-chain SPL mint
            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { mintProgramId: CTOKEN_PROGRAM_ID },
            );

            expect(result.ctokenAta).toBeDefined();
            expect(result.sources.length).toBe(0);
            expect(result.totalAmount).toBe(BigInt(0));
            expect(result.requiresProof).toBe(false);
        });

        it('should detect compressed tokens as source for CToken mint', async () => {
            const owner = Keypair.generate();

            // Mint compressed tokens
            const txId = await mintToCompressed(
                rpc,
                payer,
                mint,
                mintAuthority,
                [{ recipient: owner.publicKey, amount: 500 }],
            );
            await rpc.confirmTransaction(txId, 'confirmed');

            // For CToken mints, pass CTOKEN_PROGRAM_ID since there's no on-chain SPL mint
            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { mintProgramId: CTOKEN_PROGRAM_ID },
            );

            expect(result.sources.length).toBe(1);

            const compressedSource = result.sources.find(
                s => s.type === 'compressed',
            );
            expect(compressedSource).toBeDefined();
            expect(compressedSource!.amount).toBe(BigInt(500));

            expect(result.totalAmount).toBe(BigInt(500));
            expect(result.requiresProof).toBe(true);
        });

        it('should handle multiple compressed token accounts for CToken mint', async () => {
            const owner = Keypair.generate();

            // Mint multiple times to create multiple compressed accounts
            const tx1 = await mintToCompressed(
                rpc,
                payer,
                mint,
                mintAuthority,
                [{ recipient: owner.publicKey, amount: 100 }],
            );
            await rpc.confirmTransaction(tx1, 'confirmed');

            const tx2 = await mintToCompressed(
                rpc,
                payer,
                mint,
                mintAuthority,
                [{ recipient: owner.publicKey, amount: 200 }],
            );
            await rpc.confirmTransaction(tx2, 'confirmed');

            // For CToken mints, pass CTOKEN_PROGRAM_ID since there's no on-chain SPL mint
            const result = await loadAtaInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                owner.publicKey,
                { mintProgramId: CTOKEN_PROGRAM_ID },
            );

            expect(result.sources.length).toBe(1);
            expect(result.sources[0].type).toBe('compressed');
            expect(result.totalAmount).toBe(BigInt(300));
            expect(result.requiresProof).toBe(true);
            expect(result.compressedAccounts!.length).toBeGreaterThanOrEqual(2);
        });

        it('should correctly derive CToken ATA for CToken mint', () => {
            const owner = Keypair.generate().publicKey;
            const ctokenAta = getAtaAddressInterface(mint, owner);

            // Verify it matches expected derivation
            const expectedAta = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                CTOKEN_PROGRAM_ID,
                getAtaProgramId(CTOKEN_PROGRAM_ID),
            );

            expect(ctokenAta.toString()).toBe(expectedAta.toString());
        });
    });
});

describe('loadAtaInterface source detection', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    it('should report correct source types', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Mint and decompress some to SPL ATA
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(600),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(300),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(300)),
        );

        const result = await loadAtaInterfaceInstructions(
            rpc,
            payer.publicKey,
            mint,
            owner.publicKey,
            { tokenPoolInfos },
        );

        // Check source types
        expect(result.sources.some(s => s.type === 'spl')).toBe(true);
        expect(result.sources.some(s => s.type === 'compressed')).toBe(true);

        // Check amounts
        const splSource = result.sources.find(s => s.type === 'spl')!;
        const compressedSource = result.sources.find(
            s => s.type === 'compressed',
        )!;

        expect(splSource.amount).toBe(BigInt(300));
        expect(compressedSource.amount).toBe(BigInt(300));
        expect(result.totalAmount).toBe(BigInt(600));
    });

    it('should report correct addresses for sources', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Mint and decompress
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(400),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(200),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(200)),
        );

        const result = await loadAtaInterfaceInstructions(
            rpc,
            payer.publicKey,
            mint,
            owner.publicKey,
            { tokenPoolInfos },
        );

        // SPL source should have the correct ATA address
        const splSource = result.sources.find(s => s.type === 'spl');
        expect(splSource).toBeDefined();
        expect(splSource!.address.toString()).toBe(splAta.toString());

        // Compressed source address is the owner
        const compressedSource = result.sources.find(
            s => s.type === 'compressed',
        );
        expect(compressedSource).toBeDefined();
        expect(compressedSource!.address.toString()).toBe(
            owner.publicKey.toString(),
        );
    });
});
