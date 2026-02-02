import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
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
} from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import { transferInterface } from '../../src/v3/actions/transfer-interface';
import {
    loadAta,
    createLoadAtaInstructions,
} from '../../src/v3/actions/load-ata';
import {
    createTransferInterfaceInstruction,
    createCTokenTransferInstruction,
} from '../../src/v3/instructions/transfer-interface';

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

        it('should have owner as writable (pays for top-ups)', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createCTokenTransferInstruction(
                source,
                destination,
                owner,
                amount,
            );

            expect(ix.keys.length).toBe(3);
            expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[2].isSigner).toBe(true);
            expect(ix.keys[2].isWritable).toBe(true); // owner pays for top-ups
        });
    });

    describe('createLoadAtaInstructions', () => {
        it('should return empty when no balances to load (idempotent)', async () => {
            const owner = Keypair.generate();
            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const ixs = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
                payer.publicKey,
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

            expect(ixs.length).toBeGreaterThan(0);
        });
    });

    describe('loadAta action', () => {
        it('should return null when nothing to load (idempotent)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await loadAta(rpc, ata, owner, mint);

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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ata, owner, mint);

            expect(signature).not.toBeNull();
            expect(typeof signature).toBe('string');

            // Verify hot balance increased
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
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
            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, mint);

            // Create recipient ATA first (like SPL Token flow)
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer - destination is ATA address
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipientAta.parsed.address,
                sender,
                BigInt(1000),
            );

            expect(signature).toBeDefined();

            // Verify balances
            const senderAtaInfo = await rpc.getAccountInfo(sourceAta);
            const senderBalance = senderAtaInfo!.data.readBigUInt64LE(64);
            expect(senderBalance).toBe(BigInt(4000));

            const recipientAtaInfo = await rpc.getAccountInfo(
                recipientAta.parsed.address,
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

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer should auto-load sender's cold balance
            const signature = await transferInterface(
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

            expect(signature).toBeDefined();

            // Verify recipient received tokens
            const recipientAtaInfo = await rpc.getAccountInfo(
                recipientAta.parsed.address,
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
                    mint,
                    recipientAta.parsed.address,
                    sender,
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

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            await expect(
                transferInterface(
                    rpc,
                    payer,
                    sourceAta,
                    mint,
                    recipientAta.parsed.address,
                    sender,
                    BigInt(99999),
                    CTOKEN_PROGRAM_ID,
                    undefined,
                    { splInterfaceInfos: tokenPoolInfos },
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
            const senderAta2 = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta2, sender, mint);

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
            const recipientAta2 = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            await loadAta(
                rpc,
                recipientAta2,
                recipient,
                mint,
                undefined,
                undefined,
                {
                    splInterfaceInfos: tokenPoolInfos,
                },
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            const destAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );

            const recipientBalanceBefore = (await rpc.getAccountInfo(
                destAta,
            ))!.data.readBigUInt64LE(64);

            // Transfer
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                destAta,
                sender,
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
