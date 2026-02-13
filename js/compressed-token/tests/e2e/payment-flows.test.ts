/**
 * Payment Flows Test
 *
 * Demonstrates CToken payment patterns at both action and instruction level.
 * Mirrors SPL Token's flow: destination ATA must exist before transfer.
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
    LIGHT_TOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAtaInterface } from '../../src/v3/get-account-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import {
    transferInterface,
    createTransferInterfaceInstructions,
    sliceLast,
} from '../../src/v3/actions/transfer-interface';
import {
    createLoadAccountsParams,
    loadAta,
} from '../../src/v3/actions/load-ata';
import { createLightTokenTransferInstruction } from '../../src/v3/instructions/transfer-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../../src/v3/instructions/create-ata-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('Payment Flows', () => {
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

    // ================================================================
    // ACTION LEVEL - Mirrors SPL Token pattern
    // ================================================================

    describe('Action Level', () => {
        it('SPL Token pattern: getOrCreate + transfer', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();
            const amount = BigInt(1000);

            // Setup: mint compressed tokens to sender
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

            // STEP 1: getOrCreateAtaInterface for recipient (like SPL's getOrCreateAssociatedTokenAccount)
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // STEP 2: transfer (auto-loads sender, auto-creates recipient ATA)
            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                sender,
                amount,
                undefined,
                undefined,
                { splInterfaceInfos: tokenPoolInfos },
            );

            expect(signature).toBeDefined();

            // Verify
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta.parsed.address,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(amount);
        });

        it('sender cold, recipient no ATA', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint to sender (cold)
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

            // Create recipient ATA first
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Transfer - auto-loads sender, auto-creates recipient ATA
            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                sender,
                BigInt(2000),
                undefined,
                undefined,
                { splInterfaceInfos: tokenPoolInfos },
            );

            // Verify
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta.parsed.address,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(2000));

            const senderBalance = (await rpc.getAccountInfo(
                sourceAta,
            ))!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(1000));
        });

        it('both sender and recipient have existing hot ATAs', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // Setup both with hot balances
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

            await mintTo(
                rpc,
                payer,
                mint,
                recipient.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            await loadAta(rpc, recipientAta, recipient, mint);

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const destAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            const recipientBefore = (await rpc.getAccountInfo(
                destAta,
            ))!.data.readBigUInt64LE(64);

            // Transfer - no loading needed, pass wallet pubkey
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                sender,
                BigInt(500),
            );

            const recipientAfter = (await rpc.getAccountInfo(
                destAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientAfter).toBe(recipientBefore + BigInt(500));
        });
    });

    // ================================================================
    // INSTRUCTION LEVEL - Full control
    // ================================================================

    describe('Instruction Level', () => {
        it('manual: load + create ATA + transfer', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();
            const amount = BigInt(1000);

            // Mint to sender (cold)
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

            // STEP 1: Fetch sender's ATA for loading
            const senderAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const senderAta = await getAtaInterface(
                rpc,
                senderAtaAddress,
                sender.publicKey,
                mint,
            );

            // STEP 2: Build load params
            const result = await createLoadAccountsParams(
                rpc,
                payer.publicKey,
                LIGHT_TOKEN_PROGRAM_ID,
                [],
                [senderAta],
                { splInterfaceInfos: tokenPoolInfos },
            );

            const recipientAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            // STEP 4: Build instructions
            const instructions = [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }),
                // Load sender
                ...result.ataInstructions,
                // Create recipient ATA (idempotent)
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    recipientAtaAddress,
                    recipient.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
                // Transfer
                createLightTokenTransferInstruction(
                    senderAtaAddress,
                    recipientAtaAddress,
                    sender.publicKey,
                    amount,
                ),
            ];

            // STEP 5: Send
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(instructions, payer, blockhash, [sender]);
            const signature = await sendAndConfirmTx(rpc, tx);

            expect(signature).toBeDefined();

            // Verify
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAtaAddress,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(amount);
        });

        it('sender already hot - minimal instructions', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Setup sender hot
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
            const senderAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAtaAddress, sender, mint);

            // Sender is hot - createLoadAccountsParams returns empty ataInstructions
            const senderAta = await getAtaInterface(
                rpc,
                senderAtaAddress,
                sender.publicKey,
                mint,
            );
            const result = await createLoadAccountsParams(
                rpc,
                payer.publicKey,
                LIGHT_TOKEN_PROGRAM_ID,
                [],
                [senderAta],
            );
            expect(result.ataInstructions).toHaveLength(0);

            const recipientAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            const instructions = [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 50_000 }),
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    recipientAtaAddress,
                    recipient.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
                createLightTokenTransferInstruction(
                    senderAtaAddress,
                    recipientAtaAddress,
                    sender.publicKey,
                    BigInt(500),
                ),
            ];

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(instructions, payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            // Verify
            const balance = (await rpc.getAccountInfo(
                recipientAtaAddress,
            ))!.data.readBigUInt64LE(64);
            expect(balance).toBe(BigInt(500));
        });

        it('multiple recipients in single tx', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient1 = Keypair.generate();
            const recipient2 = Keypair.generate();

            // Setup sender hot
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(10000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, mint);

            const senderAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const r1AtaAddress = getAssociatedTokenAddressInterface(
                mint,
                recipient1.publicKey,
            );
            const r2AtaAddress = getAssociatedTokenAddressInterface(
                mint,
                recipient2.publicKey,
            );

            const instructions = [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 100_000 }),
                // Create ATAs
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    r1AtaAddress,
                    recipient1.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    r2AtaAddress,
                    recipient2.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
                // Transfers
                createLightTokenTransferInstruction(
                    senderAtaAddress,
                    r1AtaAddress,
                    sender.publicKey,
                    BigInt(1000),
                ),
                createLightTokenTransferInstruction(
                    senderAtaAddress,
                    r2AtaAddress,
                    sender.publicKey,
                    BigInt(2000),
                ),
            ];

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(instructions, payer, blockhash, [sender]);
            await sendAndConfirmTx(rpc, tx);

            // Verify
            const r1Balance = (await rpc.getAccountInfo(
                r1AtaAddress,
            ))!.data.readBigUInt64LE(64);
            const r2Balance = (await rpc.getAccountInfo(
                r2AtaAddress,
            ))!.data.readBigUInt64LE(64);
            expect(r1Balance).toBe(BigInt(1000));
            expect(r2Balance).toBe(BigInt(2000));
        });
    });

    // ================================================================
    // TRANSFER INSTRUCTIONS (Production Payment Pattern)
    // ================================================================

    describe('createTransferInterfaceInstructions', () => {
        it('hot sender: single batch', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();
            const amount = BigInt(500);

            // Setup: mint and load to make sender hot
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(2000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, mint);

            // Ensure recipient ATA exists
            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Get transfer instructions
            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                amount,
                sender.publicKey,
                recipient.publicKey,
            );

            // Hot sender: single transaction (no loads)
            expect(batches.length).toBe(1);

            // Production pattern: build tx, sign, send
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            // Verify
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(amount);
        });

        it('cold sender (<=8 inputs): single transaction', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint 3 compressed accounts
            for (let i = 0; i < 3; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    sender.publicKey,
                    mintAuthority,
                    bn(1000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            // Ensure recipient ATA exists
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
                BigInt(2500),
                sender.publicKey,
                recipient.publicKey,
            );

            // <=8 cold inputs: all fits in one transaction
            expect(batches.length).toBe(1);

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batches[0], payer, blockhash, [sender]);
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(2500));
        });

        it('cold sender (12 inputs): parallel load + sequential transfer', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint 12 compressed accounts (100 each = 1200 total)
            for (let i = 0; i < 12; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    sender.publicKey,
                    mintAuthority,
                    bn(100),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            // Ensure recipient ATA exists
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
                BigInt(1100),
                sender.publicKey,
                recipient.publicKey,
            );

            // >8 inputs: 2 batches (load + transfer)
            expect(batches.length).toBe(2);

            // Send: loads in parallel, then transfer
            const { rest: loads, last: transferIxs } = sliceLast(batches);
            const loadSigs = await Promise.all(
                loads.map(async ixs => {
                    const { blockhash } = await rpc.getLatestBlockhash();
                    const tx = buildAndSignTx(ixs, payer, blockhash, [sender]);
                    return sendAndConfirmTx(rpc, tx);
                }),
            );
            for (const sig of loadSigs) {
                expect(sig).toBeDefined();
            }

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(transferIxs, payer, blockhash, [sender]);
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            // Verify
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(1100));
        }, 120_000);

        it('cold sender (20 inputs): parallel loads + sequential transfer', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint 20 compressed accounts (50 each = 1000 total)
            for (let i = 0; i < 20; i++) {
                await mintTo(
                    rpc,
                    payer,
                    mint,
                    sender.publicKey,
                    mintAuthority,
                    bn(50),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );
            }

            // Ensure recipient ATA exists
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
                BigInt(900),
                sender.publicKey,
                recipient.publicKey,
            );

            // 20 inputs: 3 batches (8+8 loads + last 4 + transfer)
            expect(batches.length).toBe(3);

            // Send: loads in parallel, then transfer
            const { rest: loads, last: transferIxs } = sliceLast(batches);
            const loadSigs = await Promise.all(
                loads.map(async ixs => {
                    const { blockhash } = await rpc.getLatestBlockhash();
                    const tx = buildAndSignTx(ixs, payer, blockhash, [sender]);
                    return sendAndConfirmTx(rpc, tx);
                }),
            );
            for (const sig of loadSigs) {
                expect(sig).toBeDefined();
            }

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(transferIxs, payer, blockhash, [sender]);
            const sig = await sendAndConfirmTx(rpc, tx);
            expect(sig).toBeDefined();

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(900));
        }, 180_000);
    });

    // ================================================================
    // IDEMPOTENCY
    // ================================================================

    describe('Idempotency', () => {
        it('create ATA instruction is idempotent', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // Setup both with hot balances
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

            await mintTo(
                rpc,
                payer,
                mint,
                recipient.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            await loadAta(rpc, recipientAta, recipient, mint);

            const senderAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const recipientAtaAddress = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            // Include create ATA even though it exists - should not fail
            const instructions = [
                ComputeBudgetProgram.setComputeUnitLimit({ units: 50_000 }),
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    recipientAtaAddress,
                    recipient.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
                createLightTokenTransferInstruction(
                    senderAtaAddress,
                    recipientAtaAddress,
                    sender.publicKey,
                    BigInt(100),
                ),
            ];

            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(instructions, payer, blockhash, [sender]);

            // Should not throw
            await expect(sendAndConfirmTx(rpc, tx)).resolves.toBeDefined();
        });
    });
});
