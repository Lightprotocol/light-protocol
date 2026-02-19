/**
 * Multi-Cold-Inputs Batching Test Suite
 *
 * Instruction-level building and parallel multi-tx batching tests.
 * Separated from multi-cold-inputs.test.ts because these tests
 * consume ~92 output queue entries, and the combined total (~175)
 * exceeds the local test validator's 100-entry batch queue limit.
 *
 * Reset ledger/queues before running: `pnpm test-validator` (or `light test-validator`).
 * The npm script runs in two passes (validator reset between) so output-queue-heavy
 * "parallel multi-tx batching" tests get a fresh queue; use `pnpm test:e2e:multi-cold-inputs-batching`.
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
import {
    getAtaInterface,
    type AccountInterface,
} from '../../src/v3/get-account-interface';
import {
    loadAta,
    createLoadAtaInstructions,
    _buildLoadBatches,
} from '../../src/v3/actions/load-ata';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import {
    transferInterface,
    createTransferInterfaceInstructions,
    sliceLast,
} from '../../src/v3/actions/transfer-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

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

async function getCompressedAccountCount(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
): Promise<number> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, { mint });
    return result.items.length;
}

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

describe('Multi-Cold-Inputs Batching', () => {
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
    // instruction-level building (~28 output entries)
    // ---------------------------------------------------------------
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

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            expect(batches.length).toBeGreaterThan(0);
            const ixs = batches[0];

            for (let i = 0; i < ixs.length; i++) {
                const ix = ixs[i];
                console.log(`Instruction ${i}:`, {
                    programId: ix.programId.toBase58(),
                    numKeys: ix.keys.length,
                    dataLength: ix.data.length,
                });
            }

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

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            const ixs = batches[0];
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

            const serialized = tx.serialize();
            console.log('Serialized transaction size:', serialized.length);

            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            const countAfter = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(countAfter).toBe(0);
        }, 180_000);

        it('should manually build and send 2 txs with 15 cold inputs using batches (8+7 for V2)', async () => {
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

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // 15 = 8 + 7 (V2 valid proof sizes)
            expect(batches.length).toBe(2);

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

            // Batch 0: setup (createATA) + decompress 8. Batch 1: idempotent ATA + decompress 7
            // (_buildLoadBatches adds idempotent ATA to every batch after the first so order does not matter)
            expect(batches[0].length).toBe(2);
            expect(batches[1].length).toBe(2);

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

            // 15 = 8+7 = 2 batches for V2
            expect(signatures.length).toBe(2);

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
    // hash uniqueness across batches (~10 output entries)
    // ---------------------------------------------------------------
    describe('hash uniqueness across batches', () => {
        it('should partition 10 cold account hashes into non-overlapping batches', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const coldCount = 10;
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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // Get account interface (same call createTransferInterfaceInstructions makes)
            const ataInterface: AccountInterface = await getAtaInterface(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            // Build internal load batches directly to inspect compressedAccounts
            const batches = await _buildLoadBatches(
                rpc,
                payer.publicKey,
                ataInterface,
                undefined,
                false,
                ata,
            );

            expect(batches.length).toBeGreaterThan(1);

            // Collect ALL hashes across ALL batches
            const allHashes: string[] = [];
            for (const batch of batches) {
                for (const acc of batch.compressedAccounts) {
                    allHashes.push(acc.compressedAccount.hash.toString());
                }
            }

            // Every hash must be unique
            const uniqueHashes = new Set(allHashes);
            expect(uniqueHashes.size).toBe(allHashes.length);

            // Total accounts across batches must equal input count
            expect(allHashes.length).toBe(coldCount);

            console.log(
                `10 cold inputs: ${batches.length} batches, ` +
                    `accounts per batch: [${batches.map(b => b.compressedAccounts.length)}], ` +
                    `all ${allHashes.length} hashes unique: true`,
            );
        }, 120_000);

        it('should throw when duplicate compressed account hash is injected across chunks', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const coldCount = 9;
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
            const ataInterface = await getAtaInterface(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            const sources = ataInterface._sources ?? [];
            const coldSources = sources.filter(
                s =>
                    s.type === 'ctoken-cold' ||
                    s.type === 'spl-cold' ||
                    s.type === 'token2022-cold',
            );
            expect(coldSources.length).toBeGreaterThanOrEqual(9);

            const tamperedSources = [...sources, coldSources[0]];
            const tamperedInterface: AccountInterface = {
                ...ataInterface,
                _sources: tamperedSources,
            };

            await expect(
                _buildLoadBatches(
                    rpc,
                    payer.publicKey,
                    tamperedInterface,
                    undefined,
                    false,
                    ata,
                ),
            ).rejects.toThrow(
                'Duplicate compressed account hash across chunks',
            );
        }, 120_000);

        it('should transfer with 10 cold inputs using unique hashes end-to-end', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const recipient = Keypair.generate();
            const coldCount = 10;
            const amountPerAccount = BigInt(100);
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

            // createTransferInterfaceInstructions should produce
            // batches with non-overlapping hashes
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                totalAmount,
                owner.publicKey,
                recipient.publicKey,
            );

            // With 10 cold inputs: 2 batches (8+2 for V2).
            expect(batches.length).toBe(2);

            const { rest: loads, last: transferIxs } = sliceLast(batches);

            // Send load batches in parallel
            await Promise.all(
                loads.map(async ixs => {
                    const { blockhash } = await rpc.getLatestBlockhash();
                    const tx = buildAndSignTx(ixs, payer, blockhash, [owner]);
                    return sendAndConfirmTx(rpc, tx);
                }),
            );

            // Send transfer
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(transferIxs, payer, blockhash, [owner]);
            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            // Verify sender has no cold accounts left
            const senderCount = await getCompressedAccountCount(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(senderCount).toBe(0);

            // Verify recipient received tokens
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(totalAmount);
        }, 180_000);
    });

    // ---------------------------------------------------------------
    // ensureRecipientAta (default true) -- no manual ATA creation
    // ---------------------------------------------------------------
    describe('ensureRecipientAta default', () => {
        it('should create recipient ATA automatically via ensureRecipientAta (hot sender)', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const recipient = Keypair.generate();

            // Mint compressed tokens then load to make sender hot
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
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            await loadAta(rpc, senderAta, owner, mint);

            const transferAmount = BigInt(200);

            // Build instructions -- do NOT manually create recipient ATA
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                transferAmount,
                owner.publicKey,
                recipient.publicKey,
                // ensureRecipientAta defaults to true
            );

            // Hot sender: single batch with CU + recipient ATA + transfer ix
            expect(batches.length).toBe(1);
            expect(batches[0].length).toBe(3);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [owner]);
            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            // Verify recipient ATA was created and has correct balance
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientInfo = await rpc.getAccountInfo(recipientAta);
            expect(recipientInfo).not.toBeNull();
            const recipientBalance = recipientInfo!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(transferAmount);
        }, 120_000);

        it('should create recipient ATA automatically with cold inputs (10 cold)', async () => {
            const owner = await newAccountWithLamports(rpc, 3e9);
            const recipient = Keypair.generate();
            const coldCount = 10;
            const amountPerAccount = BigInt(100);
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

            // Build instructions -- no manual recipient ATA creation
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                totalAmount,
                owner.publicKey,
                recipient.publicKey,
            );

            // 10 cold: 2 batches (load + transfer)
            expect(batches.length).toBe(2);

            const { rest: loads, last: transferIxs } = sliceLast(batches);

            // Send loads in parallel
            await Promise.all(
                loads.map(async ixs => {
                    const { blockhash } = await rpc.getLatestBlockhash();
                    const tx = buildAndSignTx(ixs, payer, blockhash, [owner]);
                    return sendAndConfirmTx(rpc, tx);
                }),
            );

            // Send transfer (recipient ATA creation is embedded)
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(transferIxs, payer, blockhash, [owner]);
            const signature = await sendAndConfirmTx(rpc, tx);
            expect(signature).toBeDefined();

            // Verify recipient got the tokens
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(totalAmount);
        }, 180_000);

        it('should allow opt-out with ensureRecipientAta: false', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            // Mint compressed then load to make sender hot
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            await loadAta(rpc, senderAta, owner, mint);

            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(100),
                owner.publicKey,
                recipient.publicKey,
                { ensureRecipientAta: false },
            );

            // Single batch with CU budget + transfer ix only (no ATA ix)
            expect(batches.length).toBe(1);
            expect(batches[0].length).toBe(2);
        }, 60_000);
    });

    // ---------------------------------------------------------------
    // parallel multi-tx batching (~44 output entries)
    // ---------------------------------------------------------------
    describe('parallel multi-tx batching (>16 inputs)', () => {
        it('should load 24 cold compressed accounts via parallel batches (3 batches: 8+8+8)', async () => {
            const owner = await newAccountWithLamports(rpc, 6e9);
            const coldCount = 24;
            const amountPerAccount = BigInt(50);

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
        }, 360_000);

        it('should load 20 cold compressed accounts via parallel batches (3 batches: 8+8+4)', async () => {
            const owner = await newAccountWithLamports(rpc, 5e9);
            const coldCount = 20;
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
        }, 300_000);
    });
});
