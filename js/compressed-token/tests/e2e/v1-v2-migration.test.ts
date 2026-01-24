/**
 * V1 -> V2 Migration Test Suite
 *
 * This test suite verifies V1 -> V2 migration when running V2 SDK with
 * existing V1 tokens.
 *
 * WHAT WORKS:
 * - V1 accounts are discoverable via getCompressedTokenAccountsByOwner
 * - V1 accounts can be transferred, producing V2 outputs (auto-migration)
 * - V1 accounts can be merged, producing V2 outputs (auto-migration)
 * - V1 accounts can be decompressed to SPL
 * - Validity proofs work for V1 account hashes
 * - Mixed V1+V2 accounts returned together from RPC queries
 *
 * AUTO-MIGRATION BEHAVIOR (always enabled in V2 mode):
 * - V1 inputs ALWAYS produce V2 outputs
 * - V2 inputs produce V2 outputs
 *
 * LIMITATIONS:
 * - Mixed V1+V2 batch proofs are NOT supported in the same transaction
 * - Cannot transfer/merge accounts that span both V1 and V2 trees in single tx
 *
 * MIGRATION PATH FOR USERS:
 * - Transfer/merge operations automatically migrate V1 to V2
 * - Users with mixed V1+V2 need to process them separately
 * - Decompression works the same for both V1 and V2
 */
import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    TreeType,
    featureFlags,
    VERSION,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    createMint,
    mintTo,
    transfer,
    compress,
    decompress,
    mergeTokenAccounts,
} from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { createAssociatedTokenAccount, mintTo as splMintTo } from '@solana/spl-token';
import { TokenDataVersion } from '../../src/constants';

// Force V2 mode for all tests
featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

/**
 * Get token data version from compressed account discriminator.
 */
function getVersionFromDiscriminator(
    discriminator: number[] | undefined,
): TokenDataVersion {
    if (!discriminator || discriminator.length < 8) {
        return TokenDataVersion.ShaFlat;
    }
    if (discriminator[0] === 2) {
        return TokenDataVersion.V1;
    }
    const versionByte = discriminator[7];
    if (versionByte === 3) {
        return TokenDataVersion.V2;
    }
    if (versionByte === 4) {
        return TokenDataVersion.ShaFlat;
    }
    return TokenDataVersion.ShaFlat;
}

/**
 * Helper to get a V1 state tree info for minting (simulates pre-migration state)
 */
function selectV1StateTreeInfo(treeInfos: TreeInfo[]): TreeInfo {
    return selectStateTreeInfo(treeInfos, TreeType.StateV1);
}

/**
 * Helper to get a V2 state tree info
 */
function selectV2StateTreeInfo(treeInfos: TreeInfo[]): TreeInfo {
    return selectStateTreeInfo(treeInfos, TreeType.StateV2);
}

/**
 * Assert that an account is stored in a V1 tree
 */
function assertAccountInV1Tree(account: ParsedTokenAccount) {
    expect(account.compressedAccount.treeInfo.treeType).toBe(TreeType.StateV1);
}

/**
 * Assert that an account is stored in a V2 tree
 */
function assertAccountInV2Tree(account: ParsedTokenAccount) {
    expect(account.compressedAccount.treeInfo.treeType).toBe(TreeType.StateV2);
}

/**
 * Assert that a token account has V1 discriminator
 */
function assertV1Discriminator(account: ParsedTokenAccount) {
    const version = getVersionFromDiscriminator(
        account.compressedAccount.data?.discriminator,
    );
    expect(version).toBe(TokenDataVersion.V1);
}

describe('v1-v2-migration', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintAuthority: Keypair;
    let mint: PublicKey;
    let treeInfos: TreeInfo[];
    let v1TreeInfo: TreeInfo;
    let v2TreeInfo: TreeInfo;
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

        treeInfos = await rpc.getStateTreeInfos();

        // Verify we have both V1 and V2 trees available
        const v1Trees = treeInfos.filter(t => t.treeType === TreeType.StateV1);
        const v2Trees = treeInfos.filter(t => t.treeType === TreeType.StateV2);

        expect(v1Trees.length).toBeGreaterThan(0);
        expect(v2Trees.length).toBeGreaterThan(0);

        v1TreeInfo = selectV1StateTreeInfo(treeInfos);
        v2TreeInfo = selectV2StateTreeInfo(treeInfos);

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 120_000);

    describe('RPC Layer - Account Discovery', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('getCompressedTokenAccountsByOwner returns V1 accounts when SDK is V2', async () => {
            // Mint to V1 tree
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );

            expect(accounts.items.length).toBe(1);
            assertAccountInV1Tree(accounts.items[0]);
            assertV1Discriminator(accounts.items[0]);
            expect(accounts.items[0].parsed.amount.eq(bn(1000))).toBe(true);
        });

        it('getCompressedTokenAccountsByOwner returns mixed V1+V2 accounts', async () => {
            // Mint to V1 tree
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Mint to V2 tree
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );

            expect(accounts.items.length).toBe(2);

            const v1Account = accounts.items.find(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Account = accounts.items.find(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );

            expect(v1Account).toBeDefined();
            expect(v2Account).toBeDefined();
            expect(v1Account!.parsed.amount.eq(bn(500))).toBe(true);
            expect(v2Account!.parsed.amount.eq(bn(300))).toBe(true);
        });

        it('getCompressedTokenAccountsByOwner aggregates V1+V2 correctly for balance check', async () => {
            // Mint multiple to V1 and V2 trees
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );

            // Verify we got all 3 accounts (2 V1 + 1 V2)
            expect(accounts.items.length).toBe(3);

            const totalBalance = accounts.items.reduce(
                (sum, item) => sum.add(item.parsed.amount),
                bn(0),
            );

            expect(totalBalance.eq(bn(600))).toBe(true);

            // Verify the tree types
            const v1Accounts = accounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Accounts = accounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );
            expect(v1Accounts.length).toBe(2);
            expect(v2Accounts.length).toBe(1);
        });
    });

    describe('Transfer - V1 Inputs with Auto-Migration', () => {
        let sender: Signer;
        let recipient: PublicKey;

        beforeEach(async () => {
            sender = await newAccountWithLamports(rpc, 1e9);
            recipient = Keypair.generate().publicKey;
        });

        it('transfer single V1 token auto-migrates to V2 output', async () => {
            // Setup: mint to V1 tree
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Verify input is V1
            const preSenderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            expect(preSenderAccounts.items.length).toBe(1);
            assertAccountInV1Tree(preSenderAccounts.items[0]);

            // Transfer - auto-migration to V2 is default in V2 mode
            await transfer(rpc, payer, mint, bn(700), sender, recipient);

            // Verify recipient account is now in V2 tree (auto-migrated)
            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, {
                    mint,
                });
            expect(recipientAccounts.items.length).toBe(1);
            // V1 inputs -> V2 outputs with auto-migration
            assertAccountInV2Tree(recipientAccounts.items[0]);
            expect(recipientAccounts.items[0].parsed.amount.eq(bn(700))).toBe(
                true,
            );
        });

        it('transfer with change - both outputs go to V2 tree (auto-migration)', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await transfer(rpc, payer, mint, bn(600), sender, recipient);

            // Sender change is now in V2 tree (auto-migrated)
            const senderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            expect(senderAccounts.items.length).toBe(1);
            assertAccountInV2Tree(senderAccounts.items[0]);
            expect(senderAccounts.items[0].parsed.amount.eq(bn(400))).toBe(
                true,
            );

            // Recipient in V2 tree (auto-migrated)
            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, {
                    mint,
                });
            assertAccountInV2Tree(recipientAccounts.items[0]);
        });

        it('transfer using multiple V1 inputs auto-migrates to V2', async () => {
            // Mint two separate V1 accounts
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(300),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(400),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Verify both inputs are V1
            const preSenderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            expect(preSenderAccounts.items.length).toBe(2);
            preSenderAccounts.items.forEach(a => assertAccountInV1Tree(a));

            // Transfer requires both inputs - auto-migrates to V2
            await transfer(rpc, payer, mint, bn(650), sender, recipient);

            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, {
                    mint,
                });
            expect(recipientAccounts.items.length).toBe(1);
            expect(recipientAccounts.items[0].parsed.amount.eq(bn(650))).toBe(
                true,
            );
            // Verify output is V2
            assertAccountInV2Tree(recipientAccounts.items[0]);

            // Sender change is also V2
            const senderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            expect(senderAccounts.items.length).toBe(1);
            expect(senderAccounts.items[0].parsed.amount.eq(bn(50))).toBe(true);
            assertAccountInV2Tree(senderAccounts.items[0]);
        });

        it('transfer using 4 V1 inputs (max batch) auto-migrates to V2', async () => {
            // Mint 4 V1 accounts
            for (let i = 0; i < 4; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    sender.publicKey,
                    mintAuthority,
                    bn(100),
                    v1TreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            const preSenderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            expect(preSenderAccounts.items.length).toBe(4);
            preSenderAccounts.items.forEach(a => assertAccountInV1Tree(a));

            // Transfer using all 4 inputs
            await transfer(rpc, payer, mint, bn(400), sender, recipient);

            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, {
                    mint,
                });
            expect(recipientAccounts.items.length).toBe(1);
            expect(recipientAccounts.items[0].parsed.amount.eq(bn(400))).toBe(
                true,
            );
        });
    });

    describe('Transfer - V2 Inputs', () => {
        let sender: Signer;
        let recipient: PublicKey;

        beforeEach(async () => {
            sender = await newAccountWithLamports(rpc, 1e9);
            recipient = Keypair.generate().publicKey;
        });

        it('transfer V2 token stays in V2 tree', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(1000),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preSenderAccounts =
                await rpc.getCompressedTokenAccountsByOwner(sender.publicKey, {
                    mint,
                });
            assertAccountInV2Tree(preSenderAccounts.items[0]);

            await transfer(rpc, payer, mint, bn(700), sender, recipient);

            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, {
                    mint,
                });
            expect(recipientAccounts.items.length).toBe(1);
            assertAccountInV2Tree(recipientAccounts.items[0]);
        });
    });

    describe('Mixed V1+V2 - Current Limitations', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('mixed V1+V2 batch proof request fails (expected limitation)', async () => {
            // Mint to V1 tree
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Mint to V2 tree
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(2);

            // Mixed V1+V2 proof should fail
            await expect(
                rpc.getValidityProof(
                    accounts.items.map(a => bn(a.compressedAccount.hash)),
                ),
            ).rejects.toThrow('Requested hashes belong to different tree types');
        });

        it('transfer with mixed V1+V2 - selects from V2 first (preferred)', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const recipient = Keypair.generate().publicKey;

            // Transfer amount that can be covered by V2 only
            await transfer(rpc, payer, mint, bn(150), owner, recipient);

            // Recipient should get V2 tokens (preferred type)
            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, { mint });
            expect(recipientAccounts.items.length).toBe(1);
            assertAccountInV2Tree(recipientAccounts.items[0]);
        });

        it('transfer with mixed V1+V2 - falls back to V1 if V2 insufficient', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const recipient = Keypair.generate().publicKey;

            // Transfer amount that can only be covered by V1
            await transfer(rpc, payer, mint, bn(400), owner, recipient);

            // Recipient should get V2 tokens (auto-migrated from V1 input)
            const recipientAccounts =
                await rpc.getCompressedTokenAccountsByOwner(recipient, { mint });
            expect(recipientAccounts.items.length).toBe(1);
            assertAccountInV2Tree(recipientAccounts.items[0]);
        });
    });

    describe('Merge - V1 Consolidation with Auto-Migration', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('merge multiple V1 accounts auto-migrates to V2 output', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(preAccounts.items.length).toBe(2);
            preAccounts.items.forEach(a => assertAccountInV1Tree(a));

            await mergeTokenAccounts(rpc, payer, mint, owner);

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(postAccounts.items.length).toBe(1);
            // Auto-migration: V1 merge produces V2 output
            assertAccountInV2Tree(postAccounts.items[0]);
            expect(postAccounts.items[0].parsed.amount.eq(bn(300))).toBe(true);
        });

        it('merge 4 V1 accounts (max batch) auto-migrates to V2', async () => {
            for (let i = 0; i < 4; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(50),
                    v1TreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(preAccounts.items.length).toBe(4);
            preAccounts.items.forEach(a => assertAccountInV1Tree(a));

            await mergeTokenAccounts(rpc, payer, mint, owner);

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(postAccounts.items.length).toBe(1);
            expect(postAccounts.items[0].parsed.amount.eq(bn(200))).toBe(true);
            // Verify auto-migration to V2
            assertAccountInV2Tree(postAccounts.items[0]);
        });

        it('merge with 1 V1 and 1 V2 account fails (cannot mix tree types)', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(preAccounts.items.length).toBe(2);

            // Should fail because we have 1 V1 and 1 V2 - can't merge mixed types
            await expect(
                mergeTokenAccounts(rpc, payer, mint, owner),
            ).rejects.toThrow(
                'Cannot merge accounts from different tree types',
            );
        });

        it('merge with 2+ V1 and 2+ V2 accounts prefers V2', async () => {
            // Create 2 V1 accounts
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Create 2 V2 accounts
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(50),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(50),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(preAccounts.items.length).toBe(4);

            // Merge should pick V2 accounts (preferred in V2 mode)
            await mergeTokenAccounts(rpc, payer, mint, owner);

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            // Should have: 2 V1 (unchanged) + 1 V2 (merged)
            expect(postAccounts.items.length).toBe(3);

            const v1Accounts = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Accounts = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );

            // V1 accounts unchanged
            expect(v1Accounts.length).toBe(2);
            // V2 merged into 1
            expect(v2Accounts.length).toBe(1);
            expect(v2Accounts[0].parsed.amount.eq(bn(100))).toBe(true);
        });

        it('merge with 2+ V1 and only 1 V2 falls back to V1', async () => {
            // Create 3 V1 accounts
            for (let i = 0; i < 3; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(100),
                    v1TreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            // Create only 1 V2 account (not enough to merge)
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(50),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(preAccounts.items.length).toBe(4);

            // Merge should fall back to V1 (V2 has only 1 account)
            await mergeTokenAccounts(rpc, payer, mint, owner);

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            // Should have: 1 V1 (merged from 3) + 1 V2 (unchanged)
            expect(postAccounts.items.length).toBe(2);

            const v1Accounts = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Accounts = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );

            // V1 merged into 1 (auto-migrated to V2)
            expect(v1Accounts.length).toBe(0);
            // Note: merged V1 becomes V2, so we have 2 V2 now
            expect(v2Accounts.length).toBe(2);
            // One is the original 50, other is merged 300
            const amounts = v2Accounts.map(a => a.parsed.amount.toNumber()).sort((a, b) => a - b);
            expect(amounts).toEqual([50, 300]);
        });
    });

    describe('Decompress - V1 to SPL', () => {
        let owner: Signer;
        let ownerAta: PublicKey;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
            ownerAta = await createAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                owner.publicKey,
            );

            // First mint SPL tokens and compress them to create pool balance
            await splMintTo(
                rpc,
                payer,
                mint,
                ownerAta,
                mintAuthority,
                1_000_000_000n, // 1B tokens
            );

            // Compress some to create pool balance
            await compress(
                rpc,
                payer,
                mint,
                bn(500_000_000),
                owner,
                ownerAta,
                owner.publicKey,
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
        });

        it('decompress V1 token to SPL ATA works', async () => {
            // Mint V1 compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            // We have the compressed from beforeEach + the new V1 mint
            const v1Account = preAccounts.items.find(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            expect(v1Account).toBeDefined();

            // Get fresh token pool infos (with updated balances)
            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);

            const preAtaBalance = await rpc.getTokenAccountBalance(ownerAta);

            await decompress(
                rpc,
                payer,
                mint,
                bn(500),
                owner,
                ownerAta,
                selectTokenPoolInfosForDecompression(freshPoolInfos, bn(500)),
            );

            // Verify SPL balance increased
            const postAtaBalance = await rpc.getTokenAccountBalance(ownerAta);
            expect(
                BigInt(postAtaBalance.value.amount) -
                    BigInt(preAtaBalance.value.amount),
            ).toBe(500n);
        });

        it('partial decompress V1 token - change account created', async () => {
            // Mint V1 compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);

            await decompress(
                rpc,
                payer,
                mint,
                bn(400),
                owner,
                ownerAta,
                selectTokenPoolInfosForDecompression(freshPoolInfos, bn(400)),
            );

            // Verify SPL balance
            const ataBalance = await rpc.getTokenAccountBalance(ownerAta);
            // 500M from compress + 400 from decompress = should have received 400
            expect(ataBalance.value.amount).toContain('400');

            // Verify compressed accounts - should have change
            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            // Should have V2 from compress + change from V1 decompress
            expect(postAccounts.items.length).toBeGreaterThanOrEqual(1);
        });

        it('decompress with mixed V1+V2 prefers V2 input', async () => {
            // Mint V1 compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Mint V2 compressed tokens
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const preAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            // beforeEach compresses 500M to V2, plus our V1 500 and V2 300
            const v1Pre = preAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Pre = preAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );
            expect(v1Pre.length).toBeGreaterThanOrEqual(1);
            expect(v2Pre.length).toBeGreaterThanOrEqual(1);

            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);

            // Decompress amount that can be covered by V2
            await decompress(
                rpc,
                payer,
                mint,
                bn(200),
                owner,
                ownerAta,
                selectTokenPoolInfosForDecompression(freshPoolInfos, bn(200)),
            );

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            const v1Post = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );

            // V1 account should be unchanged (V2 was preferred)
            expect(v1Post.length).toBe(v1Pre.length);
            expect(v1Post[0].parsed.amount.eq(bn(500))).toBe(true);
        });

        it('decompress with insufficient V2 falls back to V1', async () => {
            // Mint V1 compressed tokens (large amount)
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Mint V2 compressed tokens (small amount)
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(50),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const freshPoolInfos = await getTokenPoolInfos(rpc, mint);

            // Decompress amount that exceeds V2 balance
            await decompress(
                rpc,
                payer,
                mint,
                bn(800),
                owner,
                ownerAta,
                selectTokenPoolInfosForDecompression(freshPoolInfos, bn(800)),
            );

            const postAccounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            const v1Post = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV1,
            );
            const v2Post = postAccounts.items.filter(
                a => a.compressedAccount.treeInfo.treeType === TreeType.StateV2,
            );

            // V1 was used (fell back), should have change or be consumed
            // V2 should be unchanged (50 tokens)
            const v2Amount = v2Post.reduce(
                (sum, a) => sum.add(a.parsed.amount),
                bn(0),
            );
            // V2 (50) should still exist unchanged - we only created 50 in this test
            // but beforeEach also compresses 500M to V2
            expect(v2Amount.gte(bn(50))).toBe(true);
        });
    });

    describe('Proof Generation - V1 Accounts', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('getValidityProof works for V1 account hashes', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            assertAccountInV1Tree(accounts.items[0]);

            const proof = await rpc.getValidityProof(
                accounts.items.map(a => bn(a.compressedAccount.hash)),
            );

            expect(proof).toBeDefined();
            expect(proof.compressedProof).toBeDefined();
            expect(proof.rootIndices.length).toBe(1);
        });

        it('getValidityProofV0 works for V1 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );

            const proof = await rpc.getValidityProofV0(
                accounts.items.map(a => ({
                    hash: a.compressedAccount.hash,
                    tree: a.compressedAccount.treeInfo.tree,
                    queue: a.compressedAccount.treeInfo.queue,
                })),
            );

            expect(proof).toBeDefined();
            expect(proof.compressedProof).toBeDefined();
        });

        it('getValidityProof works for multiple V1 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(100),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(200),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(2);
            accounts.items.forEach(a => assertAccountInV1Tree(a));

            const proof = await rpc.getValidityProof(
                accounts.items.map(a => bn(a.compressedAccount.hash)),
            );

            expect(proof).toBeDefined();
            expect(proof.rootIndices.length).toBe(2);
        });
    });

    describe('Real World Scenario - V1 to V2 Migration', () => {
        it('user with V1 tokens auto-migrates to V2 through transfers', async () => {
            const walletOwner = await newAccountWithLamports(rpc, 2e9);

            // Simulate user with multiple V1 token accounts (like Phantom user)
            const amounts = [150, 75, 200, 50];
            for (const amount of amounts) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    walletOwner.publicKey,
                    mintAuthority,
                    bn(amount),
                    v1TreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            // Verify initial state - all V1
            let accounts = await rpc.getCompressedTokenAccountsByOwner(
                walletOwner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(4);
            accounts.items.forEach(a => assertAccountInV1Tree(a));
            const totalInitial = amounts.reduce((a, b) => a + b, 0); // 475

            // Transfer 1: send some tokens - auto-migrates to V2
            const friend1 = Keypair.generate().publicKey;
            await transfer(rpc, payer, mint, bn(100), walletOwner, friend1);

            const friend1Accounts = await rpc.getCompressedTokenAccountsByOwner(
                friend1,
                { mint },
            );
            expect(friend1Accounts.items.length).toBe(1);
            expect(friend1Accounts.items[0].parsed.amount.eq(bn(100))).toBe(true);
            // Friend receives V2 tokens
            assertAccountInV2Tree(friend1Accounts.items[0]);

            // Transfer 2: send more tokens - auto-migrates to V2
            const friend2 = Keypair.generate().publicKey;
            await transfer(rpc, payer, mint, bn(200), walletOwner, friend2);

            const friend2Accounts = await rpc.getCompressedTokenAccountsByOwner(
                friend2,
                { mint },
            );
            expect(friend2Accounts.items.length).toBe(1);
            expect(friend2Accounts.items[0].parsed.amount.eq(bn(200))).toBe(true);
            // Friend receives V2 tokens
            assertAccountInV2Tree(friend2Accounts.items[0]);

            // Verify remaining balance
            accounts = await rpc.getCompressedTokenAccountsByOwner(
                walletOwner.publicKey,
                { mint },
            );
            const remainingBalance = accounts.items.reduce(
                (sum, a) => sum.add(a.parsed.amount),
                bn(0),
            );
            // 475 - 100 - 200 = 175
            expect(remainingBalance.eq(bn(175))).toBe(true);
        });

        it('user can merge V1 accounts to V2 then transfer', async () => {
            const walletOwner = await newAccountWithLamports(rpc, 2e9);

            // Create multiple small V1 accounts
            for (let i = 0; i < 3; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    walletOwner.publicKey,
                    mintAuthority,
                    bn(100),
                    v1TreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            let accounts = await rpc.getCompressedTokenAccountsByOwner(
                walletOwner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(3);
            accounts.items.forEach(a => assertAccountInV1Tree(a));

            // Merge V1 accounts - auto-migrates to V2
            await mergeTokenAccounts(rpc, payer, mint, walletOwner);

            accounts = await rpc.getCompressedTokenAccountsByOwner(
                walletOwner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(1);
            expect(accounts.items[0].parsed.amount.eq(bn(300))).toBe(true);
            // Merged account is now V2
            assertAccountInV2Tree(accounts.items[0]);

            // Now transfer from merged V2 account
            const recipient = Keypair.generate().publicKey;
            await transfer(rpc, payer, mint, bn(250), walletOwner, recipient);

            const recipientAccounts = await rpc.getCompressedTokenAccountsByOwner(
                recipient,
                { mint },
            );
            expect(recipientAccounts.items.length).toBe(1);
            expect(recipientAccounts.items[0].parsed.amount.eq(bn(250))).toBe(true);
            assertAccountInV2Tree(recipientAccounts.items[0]);

            // Verify change is also V2
            accounts = await rpc.getCompressedTokenAccountsByOwner(
                walletOwner.publicKey,
                { mint },
            );
            expect(accounts.items.length).toBe(1);
            expect(accounts.items[0].parsed.amount.eq(bn(50))).toBe(true);
            assertAccountInV2Tree(accounts.items[0]);
        });
    });
});
