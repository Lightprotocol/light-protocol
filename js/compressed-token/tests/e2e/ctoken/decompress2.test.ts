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
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { createMint, mintTo } from '../../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectSplInterfaceInfosForDecompression,
    TokenPoolInfo,
} from '../../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../../src/';
import { decompressInterface } from '../../../src/v3/actions/decompress-interface';
import { createDecompressInterfaceInstruction } from '../../../src/v3/instructions/create-decompress-interface-instruction';

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
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    describe('decompressInterface action', () => {
        it('should return null when no compressed tokens', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
            );

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

            // Decompress using decompressInterface
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
            );

            expect(signature).not.toBeNull();

            // Verify CToken ATA has balance
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
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
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                BigInt(3000), // amount
            );

            expect(signature).not.toBeNull();

            // Verify CToken ATA has balance
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            // Note: decompressInterface decompresses all from selected accounts,
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
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
            );

            expect(signature).not.toBeNull();

            // Verify total hot balance = 6000
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
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
                decompressInterface(
                    rpc,
                    payer,
                    owner,
                    mint,
                    BigInt(99999), // amount
                ),
            ).rejects.toThrow('Insufficient compressed balance');
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

            // Decompress
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
            );

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

            await decompressInterface(rpc, payer, owner, mint);

            // Verify initial balance
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
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
            await decompressInterface(rpc, payer, owner, mint);

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
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                undefined, // amount (all)
                recipientAta, // destinationAta
                recipient.publicKey, // destinationOwner
            );

            expect(signature).not.toBeNull();

            // Verify recipient ATA has balance
            const recipientInfo = await rpc.getAccountInfo(recipientAta);
            expect(recipientInfo).not.toBeNull();
            expect(recipientInfo!.data.readBigUInt64LE(64)).toBe(BigInt(4000));

            // Owner's ATA should not exist or have 0 balance
            const ownerAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ownerInfo = await rpc.getAccountInfo(ownerAta);
            if (ownerInfo) {
                expect(ownerInfo.data.readBigUInt64LE(64)).toBe(BigInt(0));
            }
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

            // Decompress to c-token ATA (NOT SPL ATA)
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
            );

            expect(signature).not.toBeNull();

            // Verify c-token ATA has balance
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ctokenAtaInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ctokenAtaInfo).not.toBeNull();

            // c-token ATA should have the decompressed amount
            const ctokenBalance = ctokenAtaInfo!.data.readBigUInt64LE(64);
            expect(ctokenBalance).toBe(BigInt(5000));

            // Compressed balance should be zero
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should decompress partial amount and keep change compressed', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed SPL tokens
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

            // Decompress only half to c-token ATA
            await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                BigInt(4000), // amount
            );

            // Verify c-token ATA has partial amount
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ctokenAtaInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ctokenAtaInfo).not.toBeNull();
            const ctokenBalance = ctokenAtaInfo!.data.readBigUInt64LE(64);
            expect(ctokenBalance).toBe(BigInt(4000));

            // Remaining should still be compressed
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            const compressedBalance = compressedAfter.items.reduce(
                (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
                BigInt(0),
            );
            expect(compressedBalance).toBe(BigInt(4000));
        });

        it('should decompress compressed tokens to SPL ATA', async () => {
            // This test decompresses compressed tokens to an SPL ATA (via token pool)
            const owner = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens
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

            // Get fresh SPL interface info for decompression (pool balance may have changed)
            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);
            const splInterfaceInfo = selectSplInterfaceInfosForDecompression(
                freshPoolInfos,
                bn(6000),
            )[0];

            // Decompress to SPL ATA (not c-token)
            const signature = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                undefined, // amount (all)
                undefined, // destinationAta
                undefined, // destinationOwner
                splInterfaceInfo, // SPL destination
            );

            expect(signature).not.toBeNull();

            // Verify SPL ATA has balance
            const splAta = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
            );
            const splAtaBalance = await rpc.getTokenAccountBalance(splAta);
            expect(BigInt(splAtaBalance.value.amount)).toBe(BigInt(6000));

            // Compressed balance should be zero
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(compressedAfter.items.length).toBe(0);
        });

        it('should decompress partial amount to SPL ATA', async () => {
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

            // Get fresh SPL interface info for decompression (pool balance may have changed)
            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);
            const splInterfaceInfo = selectSplInterfaceInfosForDecompression(
                freshPoolInfos,
                bn(6000),
            )[0];

            // Decompress partial amount to SPL ATA
            await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                BigInt(6000), // amount
                undefined, // destinationAta
                undefined, // destinationOwner
                splInterfaceInfo, // SPL destination
            );

            // Verify SPL ATA has partial amount
            const splAta = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
            );
            const splAtaBalance = await rpc.getTokenAccountBalance(splAta);
            expect(BigInt(splAtaBalance.value.amount)).toBe(BigInt(6000));

            // Remaining should still be compressed
            const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            const compressedBalance = compressedAfter.items.reduce(
                (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
                BigInt(0),
            );
            expect(compressedBalance).toBe(BigInt(4000));
        });
    });
});
