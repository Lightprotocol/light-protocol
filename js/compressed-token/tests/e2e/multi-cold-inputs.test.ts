/**
 * Multi-Cold-Inputs Test Suite
 *
 * Tests loading and transferring with multiple cold compressed token states.
 * Validates behavior against program constraint: MAX_INPUT_ACCOUNTS = 8
 *
 * Scenarios:
 * - 5 cold inputs: should work (within limit)
 * - 8 cold inputs: should work (at limit)
 * - 12 cold inputs: needs chunking (2 batches: 8+4)
 * - 15 cold inputs: needs chunking (2 batches: 8+7)
 *
 * These tests verify:
 * 1. load loads ALL inputs for given owner+mint, not just amount-needed
 * 2. Fits into 1 validity proof and 1 instruction (up to 8)
 * 3. Transaction size and CU constraints
 * 4. Proper error handling when exceeding limits
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
    CTOKEN_PROGRAM_ID,
    VERSION,
    featureFlags,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo } from '../../src/actions';
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
    createLoadAtaInstructionBatches,
    MAX_INPUT_ACCOUNTS,
} from '../../src/v3/actions/load-ata';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';

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

    describe('loadAta with multiple cold inputs', () => {
        it('should load 5 cold compressed accounts in single instruction', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const coldCount = 5;
            const amountPerAccount = BigInt(1000);

            // Mint 5 separate compressed accounts
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

            // Verify we have 5 cold accounts
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

            // Load all cold accounts
            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // Build instructions to inspect
            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Should have instructions (at least 1 decompress + possibly 1 create ATA)
            expect(ixs.length).toBeGreaterThan(0);
            console.log(
                `5 cold inputs: ${ixs.length} instruction(s), data sizes: ${ixs.map(ix => ix.data.length)}`,
            );

            // Execute load
            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();

            // Verify ALL cold accounts were loaded (not just some)
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            // Verify hot balance equals total cold balance
            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(totalColdBalance);
        }, 120_000);

        it('should load 8 cold compressed accounts in single instruction (at MAX_INPUT_ACCOUNTS limit)', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const coldCount = 8;
            const amountPerAccount = BigInt(500);

            // Mint 8 separate compressed accounts
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

            // Verify we have 8 cold accounts
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

            // Build instructions to inspect
            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            console.log(
                `8 cold inputs: ${ixs.length} instruction(s), data sizes: ${ixs.map(ix => ix.data.length)}`,
            );

            // Execute load - this is at the MAX_INPUT_ACCOUNTS=8 limit
            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();

            // Verify ALL 8 cold accounts were loaded
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

        it('should load 12 cold compressed accounts (2 decompress ixs in 1 tx)', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const coldCount = 12;
            const amountPerAccount = BigInt(300);

            // Mint 12 separate compressed accounts
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

            // Verify we have 12 cold accounts
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

            // Build instructions - should return 3 instructions in 1 tx:
            // 1. CreateAssociatedTokenAccountIdempotent
            // 2. Decompress chunk 1 (8 accounts)
            // 3. Decompress chunk 2 (4 accounts)
            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Should have 3 instructions (createATA + 2 decompress chunks)
            expect(ixs.length).toBe(3);

            // Execute load
            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();

            // Verify ALL 12 cold accounts were loaded
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

        it('should load 15 cold compressed accounts via batches (2 separate txs)', async () => {
            const owner = await newAccountWithLamports(rpc, 4e9);
            const coldCount = 15;
            const amountPerAccount = BigInt(200);

            // Mint 15 separate compressed accounts
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

            // Verify we have 15 cold accounts
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

            // Build instruction batches - should return 2 batches:
            // Batch 0: createATA + decompress 8 accounts (sent as tx 1)
            // Batch 1: decompress 7 accounts (sent as tx 2)
            const { batches, totalCompressedAccounts } =
                await createLoadAtaInstructionBatches(
                    rpc,
                    ata,
                    owner.publicKey,
                    mint,
                );

            console.log(
                `15 cold inputs: ${batches.length} batches, ixs per batch: ${batches.map(b => b.length)}`,
            );

            // Should have 2 batches
            expect(batches.length).toBe(2);
            expect(totalCompressedAccounts).toBe(15);

            // First batch: createATA + decompress (2 ixs)
            expect(batches[0].length).toBe(2);
            // Second batch: decompress only (1 ix)
            expect(batches[1].length).toBe(1);

            // Execute load (loadAta sends each batch as separate tx)
            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).not.toBeNull();
            console.log(
                '15 cold inputs: loadAta succeeded with signature:',
                signature,
            );

            // Verify ALL 15 cold accounts were loaded
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

            console.log(
                `After load: ${countAfter} cold remaining, hot balance: ${hotBalance}`,
            );
        }, 240_000);
    });

    describe('transferInterface with multiple cold inputs', () => {
        it('should auto-load 5 cold inputs when transferring', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();
            const coldCount = 5;
            const amountPerAccount = BigInt(1000);
            const transferAmount = BigInt(2500); // Requires multiple inputs

            // Mint 5 cold accounts to sender
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            // Create recipient ATA
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer - should auto-load all cold and then transfer
            const signature = await transferInterface(
                rpc,
                payer,
                senderAta,
                mint,
                recipientAta.parsed.address,
                sender,
                transferAmount,
            );

            expect(signature).toBeDefined();

            // Verify recipient got the tokens
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta.parsed.address,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(transferAmount);

            // Verify sender has change in hot ATA
            const senderHotBalance = (await rpc.getAccountInfo(
                senderAta,
            ))!.data.readBigUInt64LE(64);
            const expectedChange =
                BigInt(coldCount) * amountPerAccount - transferAmount;
            expect(senderHotBalance).toBe(expectedChange);

            // Verify all cold accounts were consumed
            const coldRemaining = await getCompressedAccountCount(
                rpc,
                sender.publicKey,
                mint,
            );
            expect(coldRemaining).toBe(0);
        }, 180_000);

        it('should auto-load 8 cold inputs when transferring (at limit)', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();
            const coldCount = 8;
            const amountPerAccount = BigInt(500);
            const transferAmount = BigInt(2000);

            // Mint 8 cold accounts to sender
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            // Create recipient ATA
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer - should auto-load all 8 cold and then transfer
            const signature = await transferInterface(
                rpc,
                payer,
                senderAta,
                mint,
                recipientAta.parsed.address,
                sender,
                transferAmount,
            );

            expect(signature).toBeDefined();

            // Verify recipient got the tokens
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta.parsed.address,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(transferAmount);

            // All 8 cold accounts should be consumed
            const coldRemaining = await getCompressedAccountCount(
                rpc,
                sender.publicKey,
                mint,
            );
            expect(coldRemaining).toBe(0);
        }, 180_000);

        it('should auto-load 12 cold inputs via chunking when transferring', async () => {
            const sender = await newAccountWithLamports(rpc, 3e9);
            const recipient = Keypair.generate();
            const coldCount = 12;
            const amountPerAccount = BigInt(300);
            const transferAmount = BigInt(2000);

            // Mint 12 cold accounts to sender
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                coldCount,
                amountPerAccount,
                stateTreeInfo,
                tokenPoolInfos,
            );

            const totalColdBalance = BigInt(coldCount) * amountPerAccount;

            // Create recipient ATA
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer - should auto-load all 12 cold (via 2 chunks) and then transfer
            const signature = await transferInterface(
                rpc,
                payer,
                senderAta,
                mint,
                recipientAta.parsed.address,
                sender,
                transferAmount,
            );

            expect(signature).toBeDefined();
            console.log(
                '12 cold inputs transfer: succeeded with signature:',
                signature,
            );

            // Verify recipient got the tokens
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta.parsed.address,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(transferAmount);

            // Sender should have change in hot ATA
            const senderHotBalance = (await rpc.getAccountInfo(
                senderAta,
            ))!.data.readBigUInt64LE(64);
            const expectedChange = totalColdBalance - transferAmount;
            expect(senderHotBalance).toBe(expectedChange);

            // All 12 cold accounts should be consumed
            const coldRemaining = await getCompressedAccountCount(
                rpc,
                sender.publicKey,
                mint,
            );
            expect(coldRemaining).toBe(0);
        }, 240_000);
    });

    describe('getAtaInterface aggregation', () => {
        it('should aggregate ALL cold balances in _sources', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldCount = 6;
            const amountPerAccount = BigInt(250);

            // Mint 6 cold accounts
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

            // Fetch ATA interface
            const ataInterface = await getAtaInterface(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Should have aggregated balance
            const expectedTotal = BigInt(coldCount) * amountPerAccount;
            expect(ataInterface.parsed.amount).toBe(expectedTotal);

            // _sources should contain ALL cold accounts
            const coldSources =
                ataInterface._sources?.filter(s => s.type === 'ctoken-cold') ??
                [];
            expect(coldSources.length).toBe(coldCount);

            // Each source should have loadContext
            for (const source of coldSources) {
                expect(source.loadContext).toBeDefined();
                expect(source.loadContext?.hash).toBeDefined();
                expect(source.loadContext?.treeInfo).toBeDefined();
            }

            // isCold should be true (primary source is cold since no hot exists)
            expect(ataInterface.isCold).toBe(true);

            // _needsConsolidation should be true (multiple sources)
            expect(ataInterface._needsConsolidation).toBe(true);
        }, 120_000);
    });

    describe('instruction-level building with createLoadAtaInstructions', () => {
        it('should build decompress instruction with 5 inputs', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldCount = 5;
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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Should have at least 1 instruction
            expect(ixs.length).toBeGreaterThan(0);

            // Log instruction details for debugging
            for (let i = 0; i < ixs.length; i++) {
                const ix = ixs[i];
                console.log(`Instruction ${i}:`, {
                    programId: ix.programId.toBase58(),
                    numKeys: ix.keys.length,
                    dataLength: ix.data.length,
                });
            }

            // Build and send manually to verify it works
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                [
                    ComputeBudgetProgram.setComputeUnitLimit({
                        units: 500_000,
                    }),
                    ...ixs,
                ],
                payer,
                blockhash,
                [owner],
            );

            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            // Verify all loaded
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);
        }, 120_000);

        it('should measure CU usage for 8 cold inputs', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const coldCount = 8;
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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Calculate estimated data size
            let totalDataSize = 0;
            let totalKeyCount = 0;
            for (const ix of ixs) {
                totalDataSize += ix.data.length;
                totalKeyCount += ix.keys.length;
            }

            console.log('8 cold inputs instruction stats:', {
                instructionCount: ixs.length,
                totalDataSize,
                totalKeyCount,
            });

            // Build transaction
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                [
                    ComputeBudgetProgram.setComputeUnitLimit({
                        units: 500_000,
                    }),
                    ...ixs,
                ],
                payer,
                blockhash,
                [owner],
            );

            // Log serialized tx size
            const serialized = tx.serialize();
            console.log('Serialized transaction size:', serialized.length);

            // Execute
            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            // Verify
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);
        }, 180_000);

        it('should manually build and send 2 txs with 15 cold inputs using batches', async () => {
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

            // Verify setup
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

            // Build instruction batches using createLoadAtaInstructionBatches
            // NOTE: Multiple decompress ixs in one tx causes nullification race condition,
            // so we must send each batch as a separate transaction
            const { batches, totalCompressedAccounts } =
                await createLoadAtaInstructionBatches(
                    rpc,
                    ata,
                    owner.publicKey,
                    mint,
                );

            // Must have exactly 2 batches (8+7 accounts)
            expect(batches.length).toBe(2);
            expect(totalCompressedAccounts).toBe(15);

            // Log batch details
            for (let batchIdx = 0; batchIdx < batches.length; batchIdx++) {
                const batch = batches[batchIdx];
                console.log(
                    `Batch ${batchIdx}: ${batch.length} instruction(s)`,
                );
                for (let i = 0; i < batch.length; i++) {
                    const ix = batch[i];
                    console.log(`  Instruction ${i}:`, {
                        programId: ix.programId.toBase58(),
                        numKeys: ix.keys.length,
                        dataLength: ix.data.length,
                    });
                }
            }

            // Verify batch structure
            expect(batches[0].length).toBe(2); // createATA + decompress 8
            expect(batches[1].length).toBe(1); // decompress 7

            // Manually build and send EACH batch as a separate transaction
            const signatures: string[] = [];
            for (let batchIdx = 0; batchIdx < batches.length; batchIdx++) {
                const batch = batches[batchIdx];
                const { blockhash } = await rpc.getLatestBlockhash();

                const tx = buildAndSignTx(
                    [
                        ComputeBudgetProgram.setComputeUnitLimit({
                            units: 600_000,
                        }),
                        ...batch,
                    ],
                    payer,
                    blockhash,
                    [owner],
                );

                const serialized = tx.serialize();
                console.log(
                    `Batch ${batchIdx} serialized tx size:`,
                    serialized.length,
                );

                const signature = await sendAndConfirmTx(rpc, tx);
                expect(signature).toBeDefined();
                signatures.push(signature);
                console.log(`Batch ${batchIdx} succeeded:`, signature);
            }

            expect(signatures.length).toBe(2);

            // Verify ALL 15 cold accounts were loaded
            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);

            // Verify hot balance
            const hotBalance = (await rpc.getAccountInfo(
                ata,
            ))!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(totalColdBalance);
        }, 240_000);
    });

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
});
