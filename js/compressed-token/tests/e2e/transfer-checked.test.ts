import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey, SystemProgram } from '@solana/web3.js';
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
import { transferCheckedInterface } from '../../src/v3/actions/transfer-checked';
import { loadAta } from '../../src/v3/actions/load-ata';
import {
    createTransferCheckedInterfaceInstruction,
    createCTokenTransferCheckedInstruction,
} from '../../src/v3/instructions/transfer-checked';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('transfer-checked', () => {
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

    describe('createTransferCheckedInterfaceInstruction', () => {
        it('should create c-token transfer_checked instruction with correct accounts', () => {
            const source = Keypair.generate().publicKey;
            const mintKey = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createTransferCheckedInterfaceInstruction(
                source,
                mintKey,
                destination,
                owner,
                amount,
                9,
            );

            expect(ix.programId.equals(CTOKEN_PROGRAM_ID)).toBe(true);
            expect(ix.keys.length).toBe(5);
            expect(ix.keys[0].pubkey.equals(source)).toBe(true);
            expect(ix.keys[0].isWritable).toBe(true);
            expect(ix.keys[1].pubkey.equals(mintKey)).toBe(true);
            expect(ix.keys[1].isWritable).toBe(false);
            expect(ix.keys[2].pubkey.equals(destination)).toBe(true);
            expect(ix.keys[2].isWritable).toBe(true);
            expect(ix.keys[3].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[3].isSigner).toBe(true);
            expect(ix.keys[4].pubkey.equals(SystemProgram.programId)).toBe(
                true,
            );

            // Verify discriminator (12) and 10-byte data
            expect(ix.data.length).toBe(10);
            expect(ix.data[0]).toBe(12);
        });

        it('should have authority writable when no feePayer (pays for top-ups)', () => {
            const source = Keypair.generate().publicKey;
            const mintKey = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createCTokenTransferCheckedInstruction(
                source,
                mintKey,
                destination,
                owner,
                amount,
                9,
            );

            expect(ix.keys.length).toBe(5);
            expect(ix.keys[3].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[3].isSigner).toBe(true);
            expect(ix.keys[3].isWritable).toBe(true); // writable: no feePayer
        });

        it('should have authority readonly with feePayer, and append feePayer key', () => {
            const source = Keypair.generate().publicKey;
            const mintKey = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const feePayer = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createCTokenTransferCheckedInstruction(
                source,
                mintKey,
                destination,
                owner,
                amount,
                9,
                feePayer,
            );

            expect(ix.keys.length).toBe(6);
            // Authority readonly when feePayer provided
            expect(ix.keys[3].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[3].isSigner).toBe(true);
            expect(ix.keys[3].isWritable).toBe(false);
            // feePayer is signer + writable
            expect(ix.keys[5].pubkey.equals(feePayer)).toBe(true);
            expect(ix.keys[5].isSigner).toBe(true);
            expect(ix.keys[5].isWritable).toBe(true);
        });

        it('should reject multi-signers for c-token', () => {
            const source = Keypair.generate().publicKey;
            const mintKey = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const signer = Keypair.generate();

            expect(() =>
                createTransferCheckedInterfaceInstruction(
                    source,
                    mintKey,
                    destination,
                    owner,
                    BigInt(100),
                    9,
                    [signer],
                    CTOKEN_PROGRAM_ID,
                ),
            ).toThrow('multi-signers');
        });
    });

    describe('transferCheckedInterface action', () => {
        it('should transfer with correct decimals (hot balance)', async () => {
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

            // Create recipient ATA
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

            // Transfer with correct decimals
            const signature = await transferCheckedInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipientAta.parsed.address,
                sender,
                BigInt(1000),
                TEST_TOKEN_DECIMALS,
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

        it('should reject transfer with wrong decimals', async () => {
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

            // Create recipient ATA
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

            // Transfer with WRONG decimals should fail on-chain
            await expect(
                transferCheckedInterface(
                    rpc,
                    payer,
                    sourceAta,
                    mint,
                    recipientAta.parsed.address,
                    sender,
                    BigInt(1000),
                    6, // wrong decimals (mint has 9)
                ),
            ).rejects.toThrow();
        });

        it('should auto-load cold balance before transfer', async () => {
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

            // Create recipient ATA
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
            const signature = await transferCheckedInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipientAta.parsed.address,
                sender,
                BigInt(2000),
                TEST_TOKEN_DECIMALS,
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
                transferCheckedInterface(
                    rpc,
                    payer,
                    wrongSource,
                    mint,
                    recipientAta.parsed.address,
                    sender,
                    BigInt(100),
                    TEST_TOKEN_DECIMALS,
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
                transferCheckedInterface(
                    rpc,
                    payer,
                    sourceAta,
                    mint,
                    recipientAta.parsed.address,
                    sender,
                    BigInt(99999),
                    TEST_TOKEN_DECIMALS,
                    CTOKEN_PROGRAM_ID,
                    undefined,
                    { splInterfaceInfos: tokenPoolInfos },
                ),
            ).rejects.toThrow('Insufficient balance');
        });
    });
});
