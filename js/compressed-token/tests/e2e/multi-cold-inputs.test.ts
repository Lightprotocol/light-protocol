/**
 * Multi-Cold-Inputs Test Suite
 *
 * Tests loading and transferring with multiple cold compressed token states.
 * Validates behavior against program constraint: MAX_INPUT_ACCOUNTS = 8
 *
 * Scenarios:
 * - 5 cold inputs: should work (single chunk for V2)
 * - 8 cold inputs: should work (at limit)
 * - 12 cold inputs: needs chunking (2 batches: 8+4)
 * - 15 cold inputs: needs chunking (2 batches: 8+7 for V2)
 *
 * These tests verify:
 * 1. load loads ALL inputs for given owner+mint, not just amount-needed
 * 2. Fits into 1 validity proof and 1 instruction (up to 8)
 * 3. Transaction size and CU constraints
 * 4. Proper error handling when exceeding limits
 *
 * NOTE: The local test validator has a batched output queue of 100 entries.
 * This file consumes ~83 entries. Instruction-level and parallel batching
 * tests are in multi-cold-inputs-batching.test.ts (separate validator run).
 */
import { describe, it, expect, beforeAll } from 'vitest';
import {
    Keypair,
    Signer,
    PublicKey,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    createRpc,
    selectStateTreeInfo,
    TreeInfo,
    LIGHT_TOKEN_PROGRAM_ID,
    VERSION,
    featureFlags,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, approve } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getAtaInterface } from '../../src/v3/get-account-interface';
import { transferInterface } from '../../src/v3/actions/transfer-interface';
import {
    loadAta,
    createLoadAtaInstructions,
    calculateLoadBatchComputeUnits,
    _buildLoadBatches,
    MAX_INPUT_ACCOUNTS,
} from '../../src/v3/actions/load-ata';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import {
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
} from '../../src/v3/utils/estimate-tx-size';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

/**
 * Helper to mint N separate compressed token accounts to an owner.
 * Each mint creates a distinct compressed account.
 */
async function mintMultipleColdAccounts(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    mintAuthority: Keypair,
    count: number,
    amountPerAccount: bigint,
    stateTreeInfo: TreeInfo,
    tokenPoolInfos: TokenPoolInfo[],
): Promise<void> {
    for (let i = 0; i < count; i++) {
        await mintTo(
            rpc,
            payer,
            mint,
            owner,
            mintAuthority,
            bn(amountPerAccount.toString()),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
    }
}

/**
 * Get count of compressed accounts for owner+mint
 */
async function getCompressedAccountCount(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
): Promise<number> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, { mint });
    return result.items.length;
}

/**
 * Get total compressed balance for owner+mint
 */
async function getCompressedBalance(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
): Promise<bigint> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, { mint });
    return result.items.reduce(
        (sum, item) => sum + BigInt(item.parsed.amount.toString()),
        BigInt(0),
    );
}

describe('Multi-Cold-Inputs', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 50e9);
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
    }, 120_000);

    // ---------------------------------------------------------------
    // Section 1: loadAta with multiple cold inputs (~40 output entries)
    // ---------------------------------------------------------------
    describe('loadAta with multiple cold inputs', () => {
        it('should load 5 cold compressed accounts in 1 batch, under size limit', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const coldCount = 5;
            const amountPerAccount = BigInt(1000);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const countBefore = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countBefore).toBe(coldCount);

            const totalColdBalance = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(totalColdBalance).toBe(BigInt(coldCount) * amountPerAccount);

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // 5 inputs < 8: single batch
            expect(batches.length).toBe(1);

            // Build real tx and assert serialized size
            const ixs = batches[0];
            const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
                units: 500_000,
            });
            const txIxs = [cuIx, ...ixs];
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(txIxs, payer, blockhash, [owner]);
            const serializedSize = tx.serialize().length;

            expect(serializedSize).toBeLessThanOrEqual(MAX_TRANSACTION_SIZE);

            // Cross-check estimate
            const estimate = estimateTransactionSize(txIxs, 2);
            expect(Math.abs(estimate - serializedSize)).toBeLessThanOrEqual(10);

            // Send and verify
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(totalColdBalance);
        }, 120_000);

        it('should load 8 cold compressed accounts in 1 batch at MAX_INPUT_ACCOUNTS, under size limit', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const coldCount = 8;
            const amountPerAccount = BigInt(500);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const countBefore = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countBefore).toBe(coldCount);

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // 8 inputs = exactly MAX_INPUT_ACCOUNTS: single batch
            expect(batches.length).toBe(1);

            // Build real tx and assert serialized size
            const ixs = batches[0];
            const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
                units: 500_000,
            });
            const txIxs = [cuIx, ...ixs];
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(txIxs, payer, blockhash, [owner]);
            const serializedSize = tx.serialize().length;

            expect(serializedSize).toBeLessThanOrEqual(MAX_TRANSACTION_SIZE);

            // Cross-check estimate
            const estimate = estimateTransactionSize(txIxs, 2);
            expect(Math.abs(estimate - serializedSize)).toBeLessThanOrEqual(10);

            // Send and verify
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(coldCount) * amountPerAccount);
        }, 120_000);

        it('should load 12 cold accounts in 2 txs (8+4), each under size limit', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const coldCount = 12;
            const amountPerAccount = BigInt(200);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const countBefore = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countBefore).toBe(coldCount);

            const totalColdBalance = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(totalColdBalance).toBe(BigInt(coldCount) * amountPerAccount);

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // Use createLoadAtaInstructions for both measurement and execution
            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // 12 inputs = 8+4 for V2 = 2 batches
            expect(batches.length).toBe(2);

            // Build, measure, and send each batch
            for (let i = 0; i < batches.length; i++) {
                const batchIxs = batches[i];
                const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
                    units: 500_000,
                });
                const txIxs = [cuIx, ...batchIxs];
                const { blockhash } = await rpc.getLatestBlockhash();
                const tx = buildAndSignTx(txIxs, payer, blockhash, [owner]);
                const serializedSize = tx.serialize().length;

                // Assert each batch fits within tx size limit
                expect(serializedSize).toBeLessThanOrEqual(
                    MAX_TRANSACTION_SIZE,
                );

                // Cross-check estimate is accurate
                const estimate = estimateTransactionSize(txIxs, 2);
                expect(Math.abs(estimate - serializedSize)).toBeLessThanOrEqual(
                    10,
                );

                const sig = await sendAndConfirmTx(rpc, tx);
                expect(sig).toBeDefined();
            }

            // Verify all cold accounts loaded
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(totalColdBalance);
        }, 180_000);

        it('should load 15 cold compressed accounts via batches (2 separate txs: 8+7 for V2)', async () => {
            const owner = await newAccountWithLamports(rpc, 4e9);
            const coldCount = 15;
            const amountPerAccount = BigInt(100);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const countBefore = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countBefore).toBe(coldCount);

            const totalColdBalance = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(totalColdBalance).toBe(BigInt(coldCount) * amountPerAccount);

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();

            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(totalColdBalance);
        }, 240_000);
    });

    // ---------------------------------------------------------------
    // Section 2: edge cases (~7 output entries)
    // ---------------------------------------------------------------
    describe('edge cases', () => {
        it('should handle partial load when only some accounts needed', async () => {
            // Note: Current implementation loads ALL accounts, not just needed amount
            // This test documents that behavior
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldCount = 4;
            const amountPerAccount = BigInt(1000);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // Load ATA - should load ALL 4 accounts
            await loadAta(rpc, ata, owner, mint);

            // Verify ALL accounts were loaded (not just partial)
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(coldCount) * amountPerAccount);
        }, 120_000);

        it('should be idempotent - second load returns null', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldCount = 3;
            const amountPerAccount = BigInt(500);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // First load
            const sig1 = await loadAta(rpc, ata, owner, mint);
            expect(sig1).not.toBeNull();

            // Second load - should return null (nothing to load)
            const sig2 = await loadAta(rpc, ata, owner, mint);
            expect(sig2).toBeNull();
        }, 120_000);
    });

    // ---------------------------------------------------------------
    // Section 3: delegated compressed accounts (~3 output entries)
    // ---------------------------------------------------------------
    describe('delegated compressed accounts', () => {
        it('should load compressed accounts that have delegates', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const delegate = Keypair.generate();

            // Mint compressed tokens
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

            // Approve delegate (creates a compressed account with delegate set)
            await approve(
                rpc,
                payer,
                mint,
                bn(1000),
                owner,
                delegate.publicKey,
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            // Verify compressed accounts exist with total balance preserved
            const result = await rpc.getCompressedTokenAccountsByOwner(
                owner.publicKey,
                { mint },
            );
            expect(result.items.length).toBeGreaterThan(0);
            const totalCompressed = result.items.reduce(
                (sum, item) => sum + BigInt(item.parsed.amount.toString()),
                BigInt(0),
            );
            expect(totalCompressed).toBe(BigInt(2000));

            // Verify at least one account has a delegate
            const hasDelegate = result.items.some(
                item => item.parsed.delegate !== null,
            );
            expect(hasDelegate).toBe(true);

            // Load all - should handle delegated accounts
            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();

            // Verify all loaded
            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(2000));

            const coldRemaining = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(coldRemaining).toBe(0);
        }, 120_000);
    });

    // ---------------------------------------------------------------
    // Section 4: transferInterface with cold inputs (~28 output entries)
    // ---------------------------------------------------------------
    describe('transferInterface with multiple cold inputs', () => {
        it('should auto-load 5 cold inputs when transferring', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();
            const coldCount = 5;
            const amountPerAccount = BigInt(1000);
            const totalAmount = BigInt(coldCount) * amountPerAccount;

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            // Create recipient ATA first
            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // Transfer should auto-load all cold accounts
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                owner,
                totalAmount,
            );
            expect(signature).not.toBeNull();

            // Sender should have nothing left
            const senderCount = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(senderCount).toBe(0);
        }, 120_000);

        it('should auto-load 8 cold inputs when transferring (at limit)', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();
            const coldCount = 8;
            const amountPerAccount = BigInt(500);
            const totalAmount = BigInt(coldCount) * amountPerAccount;

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                owner,
                totalAmount,
            );
            expect(signature).not.toBeNull();

            const senderCount = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(senderCount).toBe(0);
        }, 120_000);

        it('should auto-load 12 cold inputs via chunking when transferring', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const recipient = Keypair.generate();
            const coldCount = 12;
            const amountPerAccount = BigInt(200);
            const totalAmount = BigInt(coldCount) * amountPerAccount;

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                owner,
                totalAmount,
            );
            expect(signature).not.toBeNull();

            const senderCount = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(senderCount).toBe(0);
        }, 180_000);
    });

    // ---------------------------------------------------------------
    // Section 5: getAtaInterface aggregation (~5 output entries)
    // ---------------------------------------------------------------
    describe('getAtaInterface aggregation', () => {
        it('should aggregate ALL cold balances in _sources', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldCount = 5;
            const amountPerAccount = BigInt(300);

            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            // Get account interface - should aggregate all cold accounts
            const ataAddress = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ataInterface = await getAtaInterface(
                rpc,
                ataAddress,
                owner.publicKey,
                mint,
            );

            // Total aggregated amount should include all 5 cold accounts
            expect(ataInterface.parsed.amount).toBe(
                BigInt(coldCount) * amountPerAccount,
            );

            // Sources should contain 5 cold entries
            const sources = ataInterface._sources ?? [];
            const coldSources = sources.filter(
                s =>
                    s.type === 'ctoken-cold' ||
                    s.type === 'spl-cold' ||
                    s.type === 'token2022-cold',
            );
            expect(coldSources.length).toBe(coldCount);

            // Each source should have loadContext
            for (const source of coldSources) {
                expect(source.loadContext).toBeDefined();
                expect(source.loadContext!.hash).toBeDefined();
                expect(source.loadContext!.treeInfo).toBeDefined();
            }

            // isCold should be true (primary source is cold since no hot exists)
            expect(ataInterface.isCold).toBe(true);

            // _needsConsolidation should be true (multiple sources)
            expect(ataInterface._needsConsolidation).toBe(true);
        }, 120_000);
    });
});
