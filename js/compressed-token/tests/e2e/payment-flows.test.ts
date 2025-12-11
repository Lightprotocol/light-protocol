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
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
    CTOKEN_PROGRAM_ID,
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
import { getAtaInterface } from '../../src/v3/get-account-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import { transferInterface } from '../../src/v3/actions/transfer-interface';
import {
    createLoadAccountsParams,
    loadAta,
} from '../../src/v3/actions/load-ata';
import { createTransferInterfaceInstruction } from '../../src/v3/instructions/transfer-interface';
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

            // STEP 2: transfer (auto-loads sender, destination must exist)
            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipientAta.parsed.address,
                sender,
                amount,
                CTOKEN_PROGRAM_ID,
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

            // Transfer - auto-loads sender
            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipientAta.parsed.address,
                sender,
                BigInt(2000),
                CTOKEN_PROGRAM_ID,
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

            // Transfer - no loading needed
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                destAta,
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
                CTOKEN_PROGRAM_ID,
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
                    CTOKEN_PROGRAM_ID,
                ),
                // Transfer
                createTransferInterfaceInstruction(
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
                CTOKEN_PROGRAM_ID,
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
                    CTOKEN_PROGRAM_ID,
                ),
                createTransferInterfaceInstruction(
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
                    CTOKEN_PROGRAM_ID,
                ),
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    r2AtaAddress,
                    recipient2.publicKey,
                    mint,
                    CTOKEN_PROGRAM_ID,
                ),
                // Transfers
                createTransferInterfaceInstruction(
                    senderAtaAddress,
                    r1AtaAddress,
                    sender.publicKey,
                    BigInt(1000),
                ),
                createTransferInterfaceInstruction(
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
                    CTOKEN_PROGRAM_ID,
                ),
                createTransferInterfaceInstruction(
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
