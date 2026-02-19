import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    createRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectSplInterfaceInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface, loadAta } from '../../src/';
import { createDecompressInterfaceInstruction } from '../../src/v3/instructions/create-decompress-interface-instruction';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('decompressInterface', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    describe('loadAta (decompress cold to hot)', () => {
        it('should return null when no compressed tokens', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).toBeNull();
        });

        it('should decompress compressed tokens to CToken ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(5000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Verify compressed balance exists
            const compressedBefore =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });
            expect(compressedBefore.items.length).toBeGreaterThan(0);

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            const hotBalance = ataInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(5000));

            // Verify compressed balance is gone
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should load all compressed tokens to CToken ATA (loadAta)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(10000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            const hotBalance = ataInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(10000));
        });

        it('should decompress multiple compressed accounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint multiple compressed token accounts
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
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(2000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(3000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Verify multiple compressed accounts
            const compressedBefore =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });
            expect(compressedBefore.items.length).toBe(3);

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            const hotBalance = ataInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(6000));

            // Verify all compressed accounts are gone
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should load small compressed balance to CToken ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

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

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();
            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo!.data.readBigUInt64LE(64)).toBe(BigInt(100));
        });

        it('should create CToken ATA if not exists', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Verify ATA doesn't exist
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const beforeInfo = await rpc.getAccountInfo(ctokenAta);
            expect(beforeInfo).toBeNull();

            // Mint compressed tokens
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

            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const afterInfo = await rpc.getAccountInfo(ctokenAta);
            expect(afterInfo).not.toBeNull();
            const hotBalance = afterInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(1000));
        });

        it('should decompress to existing CToken ATA with balance', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint and decompress first batch
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(2000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAta, owner, mint, payer);

            const midInfo = await rpc.getAccountInfo(ctokenAta);
            expect(midInfo!.data.readBigUInt64LE(64)).toBe(BigInt(2000));

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(3000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await loadAta(rpc, ctokenAta, owner, mint, payer);

            const afterInfo = await rpc.getAccountInfo(ctokenAta);
            expect(afterInfo!.data.readBigUInt64LE(64)).toBe(BigInt(5000));
        });
    });

    describe('createDecompressInterfaceInstruction', () => {
        it('should build instruction with correct accounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens
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

            // Get compressed accounts
            const compressedResult =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });

            const proof = await rpc.getValidityProofV0(
                compressedResult.items.map(acc => ({
                    hash: acc.compressedAccount.hash,
                    tree: acc.compressedAccount.treeInfo.tree,
                    queue: acc.compressedAccount.treeInfo.queue,
                })),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ix = createDecompressInterfaceInstruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof,
                undefined,
                TEST_TOKEN_DECIMALS,
            );

            // Verify instruction structure
            expect(ix.programId.toBase58()).toBe(
                'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
            );
            expect(ix.keys.length).toBeGreaterThan(0);

            // First account should be light_system_program
            expect(ix.keys[0].pubkey.toBase58()).toBe(
                'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
            );

            // Second account should be fee payer (signer, mutable)
            expect(ix.keys[1].pubkey.equals(payer.publicKey)).toBe(true);
            expect(ix.keys[1].isSigner).toBe(true);
            expect(ix.keys[1].isWritable).toBe(true);

            // Third account should be cpi_authority_pda (not signer)
            // Owner is in packed accounts, not at index 2
            expect(ix.keys[2].isSigner).toBe(false);

            // Owner should be in packed accounts (index 7+) and marked as signer
            // Find owner in keys array (should be in packed accounts section)
            const ownerKeyIndex = ix.keys.findIndex(
                k => k.pubkey.equals(owner.publicKey) && k.isSigner,
            );
            expect(ownerKeyIndex).toBeGreaterThan(6); // After system accounts
        });

        it('should throw when no input accounts provided', () => {
            const ctokenAta = Keypair.generate().publicKey;

            expect(() =>
                createDecompressInterfaceInstruction(
                    payer.publicKey,
                    [],
                    ctokenAta,
                    BigInt(1000),
                    // Minimal mock - instruction throws before using proof
                    { compressedProof: null, rootIndices: [] } as any,
                    undefined,
                    TEST_TOKEN_DECIMALS,
                ),
            ).toThrow('No input compressed token accounts provided');
        });

        it('should handle multiple input accounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint multiple compressed token accounts
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Get compressed accounts
            const compressedResult =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });
            expect(compressedResult.items.length).toBe(2);

            const proof = await rpc.getValidityProofV0(
                compressedResult.items.map(acc => ({
                    hash: acc.compressedAccount.hash,
                    tree: acc.compressedAccount.treeInfo.tree,
                    queue: acc.compressedAccount.treeInfo.queue,
                })),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ix = createDecompressInterfaceInstruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof,
                undefined,
                TEST_TOKEN_DECIMALS,
            );

            // Instruction should be valid
            expect(ix.programId.toBase58()).toBe(
                'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
            );
            // Should have more accounts due to multiple input compressed accounts
            expect(ix.keys.length).toBeGreaterThan(10);
        });

        it('should set correct writable flags on accounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens
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

            const compressedResult =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });

            const proof = await rpc.getValidityProofV0(
                compressedResult.items.map(acc => ({
                    hash: acc.compressedAccount.hash,
                    tree: acc.compressedAccount.treeInfo.tree,
                    queue: acc.compressedAccount.treeInfo.queue,
                })),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ix = createDecompressInterfaceInstruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof,
                undefined,
                TEST_TOKEN_DECIMALS,
            );

            // Fee payer should be writable
            expect(ix.keys[1].isWritable).toBe(true);

            // Authority should not be writable
            expect(ix.keys[2].isWritable).toBe(false);

            // Find destination account and verify it's writable
            const destKey = ix.keys.find(k => k.pubkey.equals(ctokenAta));
            expect(destKey).toBeDefined();
            expect(destKey!.isWritable).toBe(true);
        });
    });

    describe('SPL mint scenarios', () => {
        it('should decompress compressed SPL tokens to c-token account', async () => {
            // This test explicitly uses an SPL mint (created via createMint with token pools)
            // to show that compressed SPL tokens can be decompressed to c-token accounts.
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed SPL tokens (from SPL mint with token pool)
            await mintTo(
                rpc,
                payer,
                mint, // SPL mint with token pool
                owner.publicKey,
                mintAuthority,
                bn(5000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Get compressed SPL token balance before
            const compressedBefore =
                await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                    mint,
                });
            const compressedBalanceBefore = compressedBefore.items.reduce(
                (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
                BigInt(0),
            );
            expect(compressedBalanceBefore).toBe(BigInt(5000));

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const ctokenAtaInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ctokenAtaInfo).not.toBeNull();
            const ctokenBalance = ctokenAtaInfo!.data.readBigUInt64LE(64);
            expect(ctokenBalance).toBe(BigInt(5000));

            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should load all compressed SPL tokens to c-token ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(8000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAta, owner, mint, payer);

            const ctokenAtaInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ctokenAtaInfo).not.toBeNull();
            const ctokenBalance = ctokenAtaInfo!.data.readBigUInt64LE(64);
            expect(ctokenBalance).toBe(BigInt(8000));

            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should load compressed tokens to SPL ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(6000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const splAta = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
            );
            const signature = await loadAta(rpc, splAta, owner, mint, payer);

            expect(signature).not.toBeNull();

            const splAtaBalance = await rpc.getTokenAccountBalance(splAta);
            expect(BigInt(splAtaBalance.value.amount)).toBe(BigInt(6000));

            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should load all compressed tokens to SPL ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(10000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const splAta = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
            );
            await loadAta(rpc, splAta, owner, mint, payer);

            const splAtaBalance = await rpc.getTokenAccountBalance(splAta);
            expect(BigInt(splAtaBalance.value.amount)).toBe(BigInt(10000));

            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });
    });
});
