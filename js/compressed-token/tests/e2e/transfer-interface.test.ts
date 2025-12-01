import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    CTOKEN_PROGRAM_ID,
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
import { getAtaAddressInterface } from '../../src/mint/actions/create-ata-interface';
import { getOrCreateAtaInterface } from '../../src/mint/actions/get-or-create-ata-interface';
import { transferInterface } from '../../src/mint/actions/transfer-interface';
import {
    loadAta,
    loadAtaInstructions,
} from '../../src/compressible/unified-load';
import {
    createTransferInterfaceInstruction,
    createCTokenTransferInstruction,
} from '../../src/mint/instructions/transfer-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('transfer-interface', () => {
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

    describe('createTransferInterfaceInstruction', () => {
        it('should create CToken transfer instruction with correct accounts', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createTransferInterfaceInstruction(
                source,
                destination,
                owner,
                amount,
            );

            expect(ix.programId.equals(CTOKEN_PROGRAM_ID)).toBe(true);
            expect(ix.keys.length).toBe(3);
            expect(ix.keys[0].pubkey.equals(source)).toBe(true);
            expect(ix.keys[1].pubkey.equals(destination)).toBe(true);
            expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
        });

        it('should add payer as 4th account when different from owner', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const payerPk = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createCTokenTransferInstruction(
                source,
                destination,
                owner,
                amount,
                payerPk,
            );

            expect(ix.keys.length).toBe(4);
            expect(ix.keys[3].pubkey.equals(payerPk)).toBe(true);
        });

        it('should not add payer when same as owner', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createCTokenTransferInstruction(
                source,
                destination,
                owner,
                amount,
                owner, // payer same as owner
            );

            expect(ix.keys.length).toBe(3);
        });
    });

    describe('loadInstructions', () => {
        it('should return empty when no balances to load (idempotent)', async () => {
            const owner = Keypair.generate();
            const ata = getAtaAddressInterface(mint, owner.publicKey);

            const ixs = await loadAtaInstructions(
                rpc,
                payer.publicKey,
                ata,
                owner.publicKey,
                mint,
            );

            expect(ixs.length).toBe(0);
        });

        it('should build load instructions for compressed balance', async () => {
            const owner = Keypair.generate();

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

            const ata = getAtaAddressInterface(mint, owner.publicKey);
            const ixs = await loadAtaInstructions(
                rpc,
                payer.publicKey,
                ata,
                owner.publicKey,
                mint,
                { tokenPoolInfos },
            );

            expect(ixs.length).toBeGreaterThan(0);
        });

        it('should load ALL compressed accounts', async () => {
            const owner = Keypair.generate();

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
                bn(300),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ata = getAtaAddressInterface(mint, owner.publicKey);
            const ixs = await loadAtaInstructions(
                rpc,
                payer.publicKey,
                ata,
                owner.publicKey,
                mint,
                { tokenPoolInfos },
            );

            expect(ixs.length).toBeGreaterThan(0);
        });
    });

    describe('loadAta action', () => {
        it('should return null when nothing to load (idempotent)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const ata = getAtaAddressInterface(mint, owner.publicKey);

            const signature = await loadAta(rpc, payer, ata, owner, mint);

            expect(signature).toBeNull();
        });

        it('should execute load and return signature', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

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

            const ata = getAtaAddressInterface(mint, owner.publicKey);
            const signature = await loadAta(
                rpc,
                payer,
                ata,
                owner,
                mint,
                undefined,
                {
                    tokenPoolInfos,
                },
            );

            expect(signature).not.toBeNull();
            expect(typeof signature).toBe('string');

            // Verify hot balance increased
            const ctokenAta = getAtaAddressInterface(mint, owner.publicKey);
            const ataInfo = await rpc.getAccountInfo(ctokenAta);
            expect(ataInfo).not.toBeNull();
            const hotBalance = ataInfo!.data.readBigUInt64LE(64);
            expect(hotBalance).toBe(BigInt(2000));
        });
    });

    describe('transferInterface action', () => {
        it('should transfer from hot balance (destination exists)', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint and load sender
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
            const senderAta = getAtaAddressInterface(mint, sender.publicKey);
            await loadAta(rpc, payer, senderAta, sender, mint, undefined, {
                tokenPoolInfos,
            });

            // Create recipient ATA first (like SPL Token flow)
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const sourceAta = getAtaAddressInterface(mint, sender.publicKey);

            // Transfer - destination is ATA address
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                recipientAta.address,
                sender,
                mint,
                BigInt(1000),
            );

            expect(signature).toBeDefined();

            // Verify balances
            const senderAtaInfo = await rpc.getAccountInfo(sourceAta);
            const senderBalance = senderAtaInfo!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(4000));

            const recipientAtaInfo = await rpc.getAccountInfo(
                recipientAta.address,
            );
            const recipientBalance = recipientAtaInfo!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(1000));
        });

        it('should auto-load sender when transferring from cold', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint compressed tokens (cold) - don't load
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

            const sourceAta = getAtaAddressInterface(mint, sender.publicKey);

            // Transfer should auto-load sender's cold balance
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                recipientAta.address,
                sender,
                mint,
                BigInt(2000),
                CTOKEN_PROGRAM_ID,
                undefined,
                { tokenPoolInfos },
            );

            expect(signature).toBeDefined();

            // Verify recipient received tokens
            const recipientAtaInfo = await rpc.getAccountInfo(
                recipientAta.address,
            );
            const recipientBalance = recipientAtaInfo!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(2000));

            // Sender should have change (loaded all 3000, sent 2000)
            const senderAtaInfo = await rpc.getAccountInfo(sourceAta);
            const senderBalance = senderAtaInfo!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(1000));
        });

        it('should throw on source mismatch', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();
            const wrongSource = Keypair.generate().publicKey;

            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            await expect(
                transferInterface(
                    rpc,
                    payer,
                    wrongSource,
                    recipientAta.address,
                    sender,
                    mint,
                    BigInt(100),
                ),
            ).rejects.toThrow('Source mismatch');
        });

        it('should throw on insufficient balance', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            // Mint small amount
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

            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const sourceAta = getAtaAddressInterface(mint, sender.publicKey);

            await expect(
                transferInterface(
                    rpc,
                    payer,
                    sourceAta,
                    recipientAta.address,
                    sender,
                    mint,
                    BigInt(99999),
                    CTOKEN_PROGRAM_ID,
                    undefined,
                    { tokenPoolInfos },
                ),
            ).rejects.toThrow('Insufficient balance');
        });

        it('should work when both sender and recipient have existing ATAs', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // Setup sender with hot balance
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
            const senderAta2 = getAtaAddressInterface(mint, sender.publicKey);
            await loadAta(rpc, payer, senderAta2, sender, mint, undefined, {
                tokenPoolInfos,
            });

            // Setup recipient with existing ATA and balance
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
            const recipientAta2 = getAtaAddressInterface(
                mint,
                recipient.publicKey,
            );
            await loadAta(
                rpc,
                payer,
                recipientAta2,
                recipient,
                mint,
                undefined,
                {
                    tokenPoolInfos,
                },
            );

            const sourceAta = getAtaAddressInterface(mint, sender.publicKey);
            const destAta = getAtaAddressInterface(mint, recipient.publicKey);

            const recipientBalanceBefore = (await rpc.getAccountInfo(
                destAta,
            ))!.data.readBigUInt64LE(64);

            // Transfer
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                destAta,
                sender,
                mint,
                BigInt(500),
            );

            // Verify recipient balance increased
            const recipientBalanceAfter = (await rpc.getAccountInfo(
                destAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalanceAfter).toBe(
                recipientBalanceBefore + BigInt(500),
            );
        });
    });
});
