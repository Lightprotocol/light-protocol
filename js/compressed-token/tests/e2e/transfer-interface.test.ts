import { describe, it, expect, beforeAll } from 'vitest';
import {
    Keypair,
    Signer,
    PublicKey,
    SystemProgram,
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
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    getAccount,
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccount,
    createFreezeAccountInstruction,
    createMintToInstruction,
    createApproveInstruction,
} from '@solana/spl-token';
import { createMint, mintTo, approve } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getOrCreateAtaInterface } from '../../src/v3/actions/get-or-create-ata-interface';
import {
    transferInterface,
    createTransferInterfaceInstructions,
} from '../../src/v3/actions/transfer-interface';
import {
    loadAta,
    createLoadAtaInstructions,
} from '../../src/v3/actions/load-ata';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { createCTokenFreezeAccountInstruction } from '../../src/v3/instructions/freeze-thaw';
import { createLightTokenTransferInstruction } from '../../src/v3/instructions/transfer-interface';
import {
    LIGHT_TOKEN_RENT_SPONSOR,
    TOTAL_COMPRESSION_COST,
    DEFAULT_PREPAY_EPOCHS,
} from '../../src/constants';
import { getAtaProgramId } from '../../src/v3/ata-utils';

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

    describe('createLightTokenTransferInstruction', () => {
        it('should create Light token transfer instruction with correct accounts', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createLightTokenTransferInstruction(
                source,
                destination,
                owner,
                amount,
            );

            expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
            // 5 accounts: source, destination, owner, system_program, fee_payer
            expect(ix.keys.length).toBe(5);
            expect(ix.keys[0].pubkey.equals(source)).toBe(true);
            expect(ix.keys[1].pubkey.equals(destination)).toBe(true);
            expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
        });

        it('should have owner as writable (pays for top-ups)', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const amount = BigInt(1000);

            const ix = createLightTokenTransferInstruction(
                source,
                destination,
                owner,
                amount,
            );

            // 5 accounts: source, destination, owner, system_program, fee_payer
            expect(ix.keys.length).toBe(5);
            expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[2].isSigner).toBe(true);
            expect(ix.keys[2].isWritable).toBe(true); // owner pays for top-ups
            // fee_payer defaults to owner
            expect(ix.keys[4].pubkey.equals(owner)).toBe(true);
            expect(ix.keys[4].isWritable).toBe(true);
        });
    });

    describe('createTransferInterfaceInstructions validation', () => {
        it('should throw when amount is zero', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate().publicKey;

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    0,
                    sender.publicKey,
                    recipient,
                ),
            ).rejects.toThrow('Transfer amount must be greater than zero.');
        });

        it('should throw when amount is negative', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate().publicKey;

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    -100,
                    sender.publicKey,
                    recipient,
                ),
            ).rejects.toThrow('Transfer amount must be greater than zero.');
        });

        it('should throw when recipient is off-curve (PDA)', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const [pdaRecipient] = PublicKey.findProgramAddressSync(
                [Buffer.from('transfer-test-pda')],
                SystemProgram.programId,
            );
            expect(PublicKey.isOnCurve(pdaRecipient.toBytes())).toBe(false);

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    BigInt(100),
                    sender.publicKey,
                    pdaRecipient,
                ),
            ).rejects.toThrow(
                'Recipient must be a wallet public key (on-curve), not a PDA or ATA',
            );
        });
    });

    describe('transferInterface frozen sender', () => {
        let splMintWithFreeze: PublicKey;
        let freezeAuthority: Keypair;

        beforeAll(async () => {
            freezeAuthority = Keypair.generate();
            const mintKeypair = Keypair.generate();
            const { mint } = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
                freezeAuthority.publicKey,
            );
            splMintWithFreeze = mint;
        }, 60_000);

        it('should throw when sender SPL token account is frozen', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate().publicKey;

            const senderSplAta = await createAssociatedTokenAccount(
                rpc,
                payer,
                splMintWithFreeze,
                sender.publicKey,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const mintIx = createMintToInstruction(
                splMintWithFreeze,
                senderSplAta,
                mintAuthority.publicKey,
                1000,
            );
            const { blockhash } = await rpc.getLatestBlockhash();
            const mintTx = buildAndSignTx([mintIx], payer, blockhash, [
                mintAuthority,
            ]);
            await sendAndConfirmTx(rpc, mintTx);

            const freezeIx = createFreezeAccountInstruction(
                senderSplAta,
                splMintWithFreeze,
                freezeAuthority.publicKey,
            );
            const freezeTx = buildAndSignTx(
                [freezeIx],
                payer,
                await rpc.getLatestBlockhash().then(b => b.blockhash),
                [freezeAuthority],
            );
            await sendAndConfirmTx(rpc, freezeTx);

            await expect(
                transferInterface(
                    rpc,
                    payer,
                    senderSplAta,
                    splMintWithFreeze,
                    recipient,
                    sender,
                    BigInt(100),
                    TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow(/Account is frozen|transfer is not allowed/);
        });

        it('should throw when sender has frozen source (wrap=true unified path)', async () => {
            const freezeAuthority = Keypair.generate();
            const mintKeypair = Keypair.generate();
            const { mint: freezableMint } = await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
                freezeAuthority.publicKey,
            );
            const freezablePoolInfos = await getTokenPoolInfos(
                rpc,
                freezableMint,
            );

            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate().publicKey;

            await mintTo(
                rpc,
                payer,
                freezableMint,
                sender.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(freezablePoolInfos),
            );
            const senderAta = getAssociatedTokenAddressInterface(
                freezableMint,
                sender.publicKey,
            );
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                freezableMint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, freezableMint, payer);

            const freezeIx = createCTokenFreezeAccountInstruction(
                senderAta,
                freezableMint,
                freezeAuthority.publicKey,
            );
            const { blockhash } = await rpc.getLatestBlockhash();
            await sendAndConfirmTx(
                rpc,
                buildAndSignTx([freezeIx], payer, blockhash, [freezeAuthority]),
            );

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    freezableMint,
                    BigInt(100),
                    sender.publicKey,
                    recipient,
                    { wrap: true },
                ),
            ).rejects.toThrow(/Account is frozen|transfer is not allowed/);

            await expect(
                transferInterface(
                    rpc,
                    payer,
                    senderAta,
                    freezableMint,
                    recipient,
                    sender,
                    BigInt(100),
                    LIGHT_TOKEN_PROGRAM_ID,
                    undefined,
                    undefined,
                    true,
                ),
            ).rejects.toThrow(/Account is frozen|transfer is not allowed/);
        });
    });

    describe('transferInterface as delegate', () => {
        it('should transfer from hot ATA when delegate is approved on ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const delegate = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            await loadAta(rpc, sourceAta, owner, mint, payer);

            const approveIx = createApproveInstruction(
                sourceAta,
                delegate.publicKey,
                owner.publicKey,
                BigInt(1000),
                [],
                LIGHT_TOKEN_PROGRAM_ID,
            );
            const { blockhash: bh } = await rpc.getLatestBlockhash();
            const approveTx = buildAndSignTx(
                [approveIx],
                payer,
                bh,
                dedupeSigner(payer, [owner]),
            );
            await sendAndConfirmTx(rpc, approveTx);

            await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
                delegate,
                BigInt(500),
                LIGHT_TOKEN_PROGRAM_ID,
                undefined,
                { owner: owner.publicKey },
            );
            expect(signature).toBeDefined();

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientInfo = await rpc.getAccountInfo(recipientAta);
            expect(recipientInfo).not.toBeNull();
            const recipientBalance = recipientInfo!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(500));
        });

        it('throws when delegate transfer needs cold approve-style sources (no CompressedOnly TLV)', async () => {
            const owner = await newAccountWithLamports(rpc, 2e9);
            const delegate = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate().publicKey;

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await approve(
                rpc,
                payer,
                mint,
                bn(1500),
                owner,
                delegate.publicKey,
            );

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    BigInt(500),
                    delegate.publicKey,
                    recipient,
                    { owner: owner.publicKey },
                ),
            ).rejects.toThrow(/delegated via approve/);
        });

        it('createTransferInterfaceInstructions throws when signer is not owner or delegate', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const delegate = await newAccountWithLamports(rpc, 1e9);
            const other = Keypair.generate().publicKey;
            const recipient = Keypair.generate().publicKey;

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
            await approve(rpc, payer, mint, bn(500), owner, delegate.publicKey);

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    BigInt(100),
                    other,
                    recipient,
                    { owner: owner.publicKey },
                ),
            ).rejects.toThrow(/Signer is not the owner or a delegate/);
        });

        it('createTransferInterfaceInstructions throws when delegate has insufficient delegated balance', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const delegate = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate().publicKey;

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
            await approve(rpc, payer, mint, bn(300), owner, delegate.publicKey);

            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    BigInt(500),
                    delegate.publicKey,
                    recipient,
                    { owner: owner.publicKey },
                ),
            ).rejects.toThrow(/Insufficient delegated balance/);
        });
    });

    describe('createLoadAtaInstructions', () => {
        it('should return empty when no balances to load (idempotent)', async () => {
            const owner = Keypair.generate();
            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
                payer.publicKey,
            );

            expect(batches.length).toBe(0);
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
            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            expect(batches.length).toBeGreaterThan(0);
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
            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
            );

            expect(batches.length).toBeGreaterThan(0);
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

            // Transfer - destination is recipient wallet public key
            const signature = await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
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
                recipient.publicKey,
                sender,
                BigInt(2000),
                undefined,
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
                    recipient.publicKey,
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
                    recipient.publicKey,
                    sender,
                    BigInt(99999),
                    undefined,
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

            // Transfer - pass recipient wallet, not ATA
            await transferInterface(
                rpc,
                payer,
                sourceAta,
                mint,
                recipient.publicKey,
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

        it('should verify ATA is funded for 24h at creation', async () => {
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

            // Get rent sponsor and payer balances before creating recipient ATA
            const rentSponsorBalanceBefore = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );
            const payerBalanceBefore = await rpc.getBalance(payer.publicKey);

            // Create recipient ATA (through getOrCreate)
            const recipientAta = await getOrCreateAtaInterface(
                rpc,
                payer,
                mint,
                recipient.publicKey,
            );

            // Get balances after ATA creation
            const rentSponsorBalanceAfter = await rpc.getBalance(
                LIGHT_TOKEN_RENT_SPONSOR,
            );
            const payerBalanceAfter = await rpc.getBalance(payer.publicKey);
            const recipientAtaBalance = await rpc.getBalance(
                recipientAta.parsed.address,
            );

            // 1) Rent sponsor pays rent exemption (~890,880 lamports)
            const rentSponsorDiff =
                rentSponsorBalanceBefore - rentSponsorBalanceAfter;
            expect(rentSponsorDiff).toBeGreaterThan(800_000);

            // 2) Fee payer pays compression_cost (11K) + 16 epochs rent + tx fees
            const payerDiff = payerBalanceBefore - payerBalanceAfter;

            // 3) Verify ATA has correct balance (rent_exemption + working capital)
            const accountInfo = await rpc.getAccountInfo(
                recipientAta.parsed.address,
            );
            const accountDataLength = accountInfo!.data.length;
            const rentExemption =
                await rpc.getMinimumBalanceForRentExemption(accountDataLength);

            // Calculate expected prepaid rent using ACTUAL account size
            const actualRentPerEpoch = 128 + accountDataLength; // base_rent + bytes * 1
            const expectedPrepaidRent =
                DEFAULT_PREPAY_EPOCHS * actualRentPerEpoch;
            const expectedFeePayerCost =
                TOTAL_COMPRESSION_COST + expectedPrepaidRent;

            expect(payerDiff).toBeGreaterThanOrEqual(expectedFeePayerCost);
            expect(payerDiff).toBeLessThan(expectedFeePayerCost + 20_000); // Allow for tx fees

            const expectedAtaBalance =
                rentExemption + TOTAL_COMPRESSION_COST + expectedPrepaidRent;

            // ATA balance should EXACTLY match (no tolerance needed when using actual size)
            expect(recipientAtaBalance).toBe(expectedAtaBalance);
        });
    });

    // ================================================================
    // H7: wrap=true + programId=TOKEN_PROGRAM_ID
    // isSplOrT22 && !wrap is false → would route to c-token transfer path,
    // but _buildLoadBatches rejects a non-ctoken targetAta when wrap=true.
    // ================================================================
    describe('H7: transferInterface wrap=true + programId=TOKEN_PROGRAM_ID', () => {
        it('should throw when wrap=true is combined with programId=TOKEN_PROGRAM_ID (targetAta is SPL)', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            // Mint compressed tokens and decompress to SPL ATA so the sender has SPL hot balance
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

            const senderSplAta = getAssociatedTokenAddressSync(
                mint,
                sender.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );
            await loadAta(rpc, senderSplAta, sender, mint, payer, undefined, {
                splInterfaceInfos: tokenPoolInfos,
            });

            // wrap=true + programId=TOKEN_PROGRAM_ID: senderAta is SPL ATA.
            // _buildLoadBatches validates targetAta is ctoken when wrap=true → throws.
            await expect(
                transferInterface(
                    rpc,
                    payer,
                    senderSplAta,
                    mint,
                    recipient.publicKey,
                    sender,
                    BigInt(500),
                    TOKEN_PROGRAM_ID,
                    undefined,
                    { splInterfaceInfos: tokenPoolInfos },
                    true, // wrap=true
                ),
            ).rejects.toThrow(/For wrap=true, ata must be the c-token ATA/);
        }, 120_000);
    });

    // ================================================================
    // H8: partially frozen cold sources, hot is unfrozen
    // Full e2e requires a freeze-compressed-account SDK operation which is
    // not yet exposed. The tests below cover what can be verified without it:
    // - hot-only sender with insufficient balance reports no frozen note
    // - unfrozen cold load path works correctly (covered by auto-load test)
    // ================================================================
    describe('H8: unfrozen balance calc – frozen balance note in error', () => {
        it('should report no frozen note when all sources are unfrozen and balance is insufficient', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

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

            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );
            await loadAta(rpc, senderAta, sender, mint);

            // Try to transfer more than available; no frozen accounts → no "frozen, not usable" note
            await expect(
                createTransferInterfaceInstructions(
                    rpc,
                    payer.publicKey,
                    mint,
                    BigInt(99_999),
                    sender.publicKey,
                    recipient.publicKey,
                ),
            ).rejects.toThrow(
                /Insufficient balance.*Required: 99999.*Available: 100$/,
            );
        }, 90_000);

        it('should succeed transferring exactly the unfrozen balance (cold-only, no frozen)', async () => {
            const sender = await newAccountWithLamports(rpc, 1e9);
            const recipient = Keypair.generate();

            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(1500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const senderAta = getAssociatedTokenAddressInterface(
                mint,
                sender.publicKey,
            );

            // Transfer exactly the cold balance - auto-load should succeed
            const signature = await transferInterface(
                rpc,
                payer,
                senderAta,
                mint,
                recipient.publicKey,
                sender,
                BigInt(1500),
            );

            expect(signature).toBeDefined();

            const recipientAta = getAssociatedTokenAddressInterface(
                mint,
                recipient.publicKey,
            );
            const recipientBalance = (await rpc.getAccountInfo(
                recipientAta,
            ))!.data.readBigUInt64LE(64);
            expect(recipientBalance).toBe(BigInt(1500));
        }, 120_000);
    });

    // ================================================================
    // SPL/T22 NO-WRAP TRANSFER (programId=TOKEN_PROGRAM_ID, wrap=false)
    // ================================================================
    describe('transferInterface with SPL programId (no-wrap)', () => {
        it('should transfer cold-only via SPL (decompress + SPL transferChecked)', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // Mint compressed tokens (cold)
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

            // Derive SPL ATAs (not c-token ATAs)
            const senderSplAta = getAssociatedTokenAddressSync(
                mint,
                sender.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );
            const recipientSplAta = getAssociatedTokenAddressSync(
                mint,
                recipient.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );

            // Transfer using SPL program (no wrap)
            // This should: 1) create sender SPL ATA, 2) decompress cold -> SPL ATA,
            // 3) create recipient SPL ATA, 4) SPL transferChecked
            const signature = await transferInterface(
                rpc,
                payer,
                senderSplAta,
                mint,
                recipient.publicKey,
                sender,
                BigInt(2000),
                TOKEN_PROGRAM_ID,
                undefined,
                { splInterfaceInfos: tokenPoolInfos },
                false,
            );

            expect(signature).toBeDefined();

            // Verify recipient SPL ATA has tokens
            const recipientAccount = await getAccount(
                rpc,
                recipientSplAta,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(recipientAccount.amount).toBe(BigInt(2000));

            // Verify sender SPL ATA has remaining tokens
            const senderAccount = await getAccount(
                rpc,
                senderSplAta,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(senderAccount.amount).toBe(BigInt(3000));
        }, 120_000);

        it('should build SPL transfer instructions via createTransferInterfaceInstructions', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = Keypair.generate();

            // Mint compressed tokens (cold)
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

            const batches = await createTransferInterfaceInstructions(
                rpc,
                payer.publicKey,
                mint,
                BigInt(1000),
                sender.publicKey,
                recipient.publicKey,
                {
                    programId: TOKEN_PROGRAM_ID,
                    splInterfaceInfos: tokenPoolInfos,
                },
            );

            // Should have at least one batch with the transfer
            expect(batches.length).toBeGreaterThan(0);

            // The last batch (transfer tx) should contain a SPL transferChecked
            // instruction as its last ix (programId = TOKEN_PROGRAM_ID)
            const transferBatch = batches[batches.length - 1];
            const transferIx = transferBatch[transferBatch.length - 1];
            expect(transferIx.programId.equals(TOKEN_PROGRAM_ID)).toBe(true);
        }, 120_000);

        it('should transfer hot-only SPL balance (no decompress needed)', async () => {
            const sender = await newAccountWithLamports(rpc, 2e9);
            const recipient = await newAccountWithLamports(rpc, 1e9);

            // First: mint compressed and decompress to SPL ATA to get hot SPL balance
            await mintTo(
                rpc,
                payer,
                mint,
                sender.publicKey,
                mintAuthority,
                bn(4000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const senderSplAta = getAssociatedTokenAddressSync(
                mint,
                sender.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );

            // Load to SPL ATA first (decompress)
            await loadAta(rpc, senderSplAta, sender, mint, payer);

            // Verify sender has hot SPL balance
            const senderBefore = await getAccount(
                rpc,
                senderSplAta,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(senderBefore.amount).toBe(BigInt(4000));

            // Now transfer using SPL programId -- should be hot-only (no decompress)
            const signature = await transferInterface(
                rpc,
                payer,
                senderSplAta,
                mint,
                recipient.publicKey,
                sender,
                BigInt(1500),
                TOKEN_PROGRAM_ID,
                undefined,
                undefined,
                false,
            );

            expect(signature).toBeDefined();

            // Verify balances
            const recipientSplAta = getAssociatedTokenAddressSync(
                mint,
                recipient.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                getAtaProgramId(TOKEN_PROGRAM_ID),
            );
            const recipientAccount = await getAccount(
                rpc,
                recipientSplAta,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(recipientAccount.amount).toBe(BigInt(1500));

            const senderAfter = await getAccount(
                rpc,
                senderSplAta,
                undefined,
                TOKEN_PROGRAM_ID,
            );
            expect(senderAfter.amount).toBe(BigInt(2500));
        }, 120_000);
    });
});
