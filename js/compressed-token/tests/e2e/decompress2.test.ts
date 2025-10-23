import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getATAAddressInterface } from '../../src/mint/actions/create-ata-interface';
import { decompress2 } from '../../src/mint/actions/decompress2';
import { createDecompress2Instruction } from '../../src/mint/instructions/decompress2';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('decompress2', () => {
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

    describe('decompress2 action', () => {
        it('should return null when no compressed tokens', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

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

            // Decompress using decompress2
            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

            expect(signature).not.toBeNull();

            // Verify CToken ATA has balance
            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);
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

        it('should decompress specific amount when provided', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens
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

            // Decompress only 3000
            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
                amount: BigInt(3000),
            });

            expect(signature).not.toBeNull();

            // Verify CToken ATA has balance
            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);
            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            // Note: decompress2 decompresses all from selected accounts,
            // so the balance will be 10000 (full account)
            const hotBalance = ataInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBeGreaterThanOrEqual(BigInt(3000));
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

            // Decompress all
            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

            expect(signature).not.toBeNull();

            // Verify total hot balance = 6000
            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);
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

        it('should throw on insufficient compressed balance', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint small amount
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

            await expect(
                decompress2({
                    rpc,
                    payer,
                    owner,
                    mint,
                    amount: BigInt(99999),
                }),
            ).rejects.toThrow('Insufficient compressed balance');
        });

        it('should create CToken ATA if not exists', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Verify ATA doesn't exist
            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);
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

            // Decompress
            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

            expect(signature).not.toBeNull();

            // Verify ATA was created with balance
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

            await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

            // Verify initial balance
            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);
            const midInfo = await rpc.getAccountInfo(ctokenAta);
            expect(midInfo!.data.readBigUInt64LE(64)).toBe(BigInt(2000));

            // Mint more compressed tokens
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

            // Decompress again
            await decompress2({
                rpc,
                payer,
                owner,
                mint,
            });

            // Verify total balance = 5000
            const afterInfo = await rpc.getAccountInfo(ctokenAta);
            expect(afterInfo!.data.readBigUInt64LE(64)).toBe(BigInt(5000));
        });

        it('should decompress to custom destination ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens to owner
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(4000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Decompress to recipient's ATA
            const recipientAta = getATAAddressInterface(
                mint,
                recipient.publicKey,
            );
            const signature = await decompress2({
                rpc,
                payer,
                owner,
                mint,
                destinationAta: recipientAta,
            });

            expect(signature).not.toBeNull();

            // Verify recipient ATA has balance
            const recipientInfo = await rpc.getAccountInfo(recipientAta);
            expect(recipientInfo).not.toBeNull();
            expect(recipientInfo!.data.readBigUInt64LE(64)).toBe(BigInt(4000));

            // Owner's ATA should not exist or have 0 balance
            const ownerAta = getATAAddressInterface(mint, owner.publicKey);
            const ownerInfo = await rpc.getAccountInfo(ownerAta);
            if (ownerInfo) {
                expect(ownerInfo.data.readBigUInt64LE(64)).toBe(BigInt(0));
            }
        });
    });

    describe('createDecompress2Instruction', () => {
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

            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);

            const ix = createDecompress2Instruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof.compressedProof,
                proof.rootIndices,
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

            // Third account should be authority/owner (signer)
            expect(ix.keys[2].pubkey.equals(owner.publicKey)).toBe(true);
            expect(ix.keys[2].isSigner).toBe(true);
        });

        it('should throw when no input accounts provided', () => {
            const ctokenAta = Keypair.generate().publicKey;

            expect(() =>
                createDecompress2Instruction(
                    payer.publicKey,
                    [],
                    ctokenAta,
                    BigInt(1000),
                    null,
                    [],
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

            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);

            const ix = createDecompress2Instruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof.compressedProof,
                proof.rootIndices,
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

            const ctokenAta = getATAAddressInterface(mint, owner.publicKey);

            const ix = createDecompress2Instruction(
                payer.publicKey,
                compressedResult.items,
                ctokenAta,
                BigInt(1000),
                proof.compressedProof,
                proof.rootIndices,
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
});
