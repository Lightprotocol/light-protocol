/**
 * Input Selection Test Suite
 *
 * Tests amount-aware greedy input selection in createTransferInterfaceInstructions.
 * Verifies that only the cold inputs needed for the transfer amount are loaded,
 * padded to MAX_INPUT_ACCOUNTS (8) when within a single batch.
 *
 * Key behavioral changes tested:
 * - 20 cold inputs, small transfer: 1 tx (8 selected) instead of 3 txs (all 20)
 * - 20 cold inputs, large transfer: 3 txs (all needed) -- unchanged
 * - Hot balance sufficient: 0 loads
 * - SPL wraps reduce cold inputs needed
 *
 * Every test asserts:
 * 1. Batch count matches expected value
 * 2. Each batch serializes within MAX_TRANSACTION_SIZE
 * 3. estimateTransactionSize cross-checks against actual serialized size
 * 4. Recipient receives the correct amount
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
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import {
    createTransferInterfaceInstructions,
    sliceLast,
} from '../../src/v3/actions/transfer-interface';
import { loadAta } from '../../src/v3/actions/load-ata';
import {
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
} from '../../src/v3/utils/estimate-tx-size';

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

/**
 * Assert that every batch in the result fits within MAX_TRANSACTION_SIZE.
 * Builds a real VersionedTransaction for each batch to get the actual
 * serialized size, and cross-checks against estimateTransactionSize.
 */
async function assertAllBatchesFitInTx(
    rpc: Rpc,
    batches: any[][],
    payer: Signer,
    signers: Signer[],
): Promise<void> {
    for (let i = 0; i < batches.length; i++) {
        const ixs = batches[i];
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(ixs, payer, blockhash, signers);
        const serialized = tx.serialize().length;

        expect(serialized).toBeLessThanOrEqual(MAX_TRANSACTION_SIZE);

        // Cross-check estimate (payer + signers, deduplicated)
        const allSignerKeys = new Set([
            payer.publicKey.toBase58(),
            ...signers.map(s => s.publicKey.toBase58()),
        ]);
        const estimate = estimateTransactionSize(ixs, allSignerKeys.size);
        expect(Math.abs(estimate - serialized)).toBeLessThanOrEqual(10);
    }
}

describe('Input Selection', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
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

    describe('createTransferInterfaceInstructions with amount-aware selection', () => {
        it('0 cold inputs (hot only): 1 batch, no loads', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint and load to make sender hot
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(5000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, mint);

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(1000),
                sender.publicKey,
                recipient.publicKey,
            );

            // Hot sender: single transfer tx, no loads
            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            // Send and verify
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(1000));
        }, 120_000);

        it('1 cold input: 1 batch (load + transfer combined)', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(3000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(2000),
                sender.publicKey,
                recipient.publicKey,
            );

            // 1 cold input fits in single batch with transfer
            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(2000));
        }, 120_000);

        it('8 cold inputs, small transfer: 1 batch (all 8 loaded, pads to fill)', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // 8 inputs of 1000 each = 8000 total
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                8,
                1000n,
                stateTreeInfo,
                tokenPoolInfos,
            );

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Transfer only 500 (1 input would suffice, but pads to 8)
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(500),
                sender.publicKey,
                recipient.publicKey,
            );

            // All 8 loaded (padding fills to MAX_INPUT_ACCOUNTS), combined with transfer
            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(500));
        }, 120_000);

        it('20 cold inputs, small transfer (needs <=8): 1 batch instead of 3', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            // 20 inputs of 1000 each = 20000 total
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                20,
                1000n,
                stateTreeInfo,
                tokenPoolInfos,
            );

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Transfer 500: only 1 input needed, pads to 8.
            // _buildLoadBatches returns 1 internal batch (8 inputs).
            // Assembly combines load + transfer = 1 tx.
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(500),
                sender.publicKey,
                recipient.publicKey,
            );

            // KEY BEHAVIORAL CHANGE: 1 batch instead of 3
            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(500));

            // Sender should have loaded 8 * 1000 = 8000, sent 500, change = 7500
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const senderBalance = (await rpc.getAccountInfo(
                senderAta,
            ))!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(7500));
        }, 240_000);

        it('20 cold inputs, large transfer (needs all): 3 batches (unchanged)', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            // 20 inputs of 50 each = 1000 total
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                20,
                50n,
                stateTreeInfo,
                tokenPoolInfos,
            );

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Transfer 900: needs 18 inputs (18*50=900), selects all 20.
            // 20 inputs -> 3 internal batches (8+8+4) -> 3 txs
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(900),
                sender.publicKey,
                recipient.publicKey,
            );

            expect(batches.length).toBe(3);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            // Send: loads in parallel, then transfer
            const { rest: loads, last: transferIxs } = sliceLast(batches);
            await Promise.all(
                loads.map(async ixs => {
                    const { blockhash } = await rpc.getLatestBlockhash();
                    const tx = buildAndSignTx(ixs, payer, blockhash, [sender]);
                    return sendAndConfirmTx(rpc, tx);
                }),
            );
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(transferIxs, payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(900));
        }, 240_000);

        it('ATA creation mixed in: included in batch alongside selected inputs', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // 3 cold inputs
            await mintMultipleColdAccounts(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                3,
                2000n,
                stateTreeInfo,
                tokenPoolInfos,
            );

            // Do NOT create recipient ATA -- let transfer create it
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(1000),
                sender.publicKey,
                recipient.publicKey,
                // ensureRecipientAta defaults to true
            );

            // 3 cold inputs -> 1 internal batch, combined with transfer + ATA creation
            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            // Verify recipient ATA was created and has correct balance
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(1000));
        }, 120_000);

        it('selection sufficiency: exact amount covered by selected inputs', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // 10 inputs with varying amounts (descending: 1000, 900, ..., 100)
            for (let i = 0; i < 10; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    sender.publicKey,
                    mintAuthority,
                    bn((1000 - i * 100).toString()),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Transfer 2500: needs 1000+900+800 = 2700 >= 2500 (3 inputs).
            // Pads to 8 since only 1 batch needed.
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(2500),
                sender.publicKey,
                recipient.publicKey,
            );

            expect(batches.length).toBe(1);
            await assertAllBatchesFitInTx(rpc, batches, payer, [sender]);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(2500));

            // Sender loaded 8 inputs (top 8 by amount: 1000+900+...+300 = 5200),
            // sent 2500, change = 2700
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const senderBalance = (await rpc.getAccountInfo(
                senderAta,
            ))!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(2700));
        }, 180_000);
    });
});
