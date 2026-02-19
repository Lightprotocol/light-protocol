import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
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
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAccount,
    createAssociatedTokenAccount,
} from '@solana/spl-token';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { createUnwrapInstruction } from '../../src/v3/instructions/unwrap';
import { unwrap, createUnwrapInstructions } from '../../src/v3/actions/unwrap';
import { createCTokenFreezeAccountInstruction } from '../../src/v3/instructions/freeze-thaw';
import { getAssociatedTokenAddressInterface } from '../../src';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { getAtaProgramId } from '../../src/v3/ata-utils';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

async function getCTokenBalance(rpc: Rpc, address: PublicKey): Promise<bigint> {
    const accountInfo = await rpc.getAccountInfo(address);
    if (!accountInfo) {
        return BigInt(0);
    }
    return accountInfo.data.readBigUInt64LE(64);
}

describe('createUnwrapInstruction', () => {
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

    it('should create valid instruction with all required params', async () => {
        const owner = Keypair.generate();
        const source = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const destination = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);
        expect(tokenPoolInfo).toBeDefined();

        const ix = createUnwrapInstruction(
            source,
            destination,
            owner.publicKey,
            mint,
            BigInt(1000),
            tokenPoolInfo!,
            TEST_TOKEN_DECIMALS,
        );

        expect(ix).toBeDefined();
        expect(ix.programId).toBeDefined();
        expect(ix.keys.length).toBeGreaterThan(0);
        expect(ix.data.length).toBeGreaterThan(0);
    });

    it('should create instruction with explicit payer', async () => {
        const owner = Keypair.generate();
        const feePayer = Keypair.generate();
        const source = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const destination = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const ix = createUnwrapInstruction(
            source,
            destination,
            owner.publicKey,
            mint,
            BigInt(500),
            tokenPoolInfo!,
            TEST_TOKEN_DECIMALS,
            feePayer.publicKey,
        );

        expect(ix).toBeDefined();
        const payerKey = ix.keys.find(
            k => k.pubkey.equals(feePayer.publicKey) && k.isSigner,
        );
        expect(payerKey).toBeDefined();
    });
});

describe('createUnwrapInstructions', () => {
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

    it('should return instruction batches including unwrap (from cold)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens (cold)
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

        // Create destination SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const batches = await createUnwrapInstructions(
            rpc,
            splAta,
            owner.publicKey,
            mint,
            BigInt(500),
            payer.publicKey,
        );

        // Should have at least one batch (load + unwrap, or just unwrap)
        expect(batches.length).toBeGreaterThanOrEqual(1);
        // Each batch should be a non-empty array of instructions
        for (const batch of batches) {
            expect(batch.length).toBeGreaterThan(0);
        }

        // Execute all batches
        for (const ixs of batches) {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, [owner]);
            await sendAndConfirmTx(rpc, tx);
        }

        // Verify SPL balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(500));

        // Verify remaining c-token balance
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(500));
    }, 60_000);

    it('should return single batch when balance is already hot', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create c-token ATA and mint to hot
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(800),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Load to hot first
        const { loadAta } = await import('../../src/v3/actions/load-ata');
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // Create destination SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const batches = await createUnwrapInstructions(
            rpc,
            splAta,
            owner.publicKey,
            mint,
            BigInt(300),
            payer.publicKey,
        );

        // Should be a single batch (no load needed, just unwrap)
        expect(batches.length).toBe(1);

        // Execute
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(batches[0], payer, blockhash, [owner]);
        await sendAndConfirmTx(rpc, tx);

        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(300));
    }, 60_000);

    it('should throw when destination does not exist', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        const splAta = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(50),
                payer.publicKey,
            ),
        ).rejects.toThrow(/does not exist/);
    }, 60_000);

    it('should throw on insufficient balance', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(99999),
                payer.publicKey,
            ),
        ).rejects.toThrow(/Insufficient/);
    }, 60_000);

    it('should throw when all c-token balance is frozen', async () => {
        // This test needs a mint with a freeze authority set.
        const freezeAuthority = Keypair.generate();
        const mintWithFreezeKeypair = Keypair.generate();
        const { mint: freezableMint } = await createMint(
            rpc,
            payer,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintWithFreezeKeypair,
            undefined,
            TOKEN_PROGRAM_ID,
            freezeAuthority.publicKey,
        );
        const freezableMintPoolInfos = await getTokenPoolInfos(
            rpc,
            freezableMint,
        );

        const owner = await newAccountWithLamports(rpc, 1e9);

        await mintTo(
            rpc,
            payer,
            freezableMint,
            owner.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            selectTokenPoolInfo(freezableMintPoolInfos),
        );

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            freezableMint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            freezableMint,
            owner.publicKey,
        );
        const { loadAta } = await import('../../src/v3/actions/load-ata');
        await loadAta(rpc, ctokenAta, owner, freezableMint, payer);

        // Freeze the hot c-token ATA
        const freezeIx = createCTokenFreezeAccountInstruction(
            ctokenAta,
            freezableMint,
            freezeAuthority.publicKey,
        );
        const { blockhash: fh } = await rpc.getLatestBlockhash();
        const freezeTx = buildAndSignTx([freezeIx], payer, fh, [
            freezeAuthority,
        ]);
        await sendAndConfirmTx(rpc, freezeTx);

        // Verify account is frozen (state byte 108 == 2)
        const accountInfo = await rpc.getAccountInfo(ctokenAta);
        expect(accountInfo).not.toBeNull();
        expect(accountInfo!.data[108]).toBe(2);

        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                freezableMint,
                undefined,
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);

        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                freezableMint,
                undefined,
                payer.publicKey,
                undefined,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);
    }, 90_000);
});

describe('unwrap action', () => {
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

    it('should unwrap c-tokens to SPL ATA (from cold)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens (cold)
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

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Unwrap to SPL (should consolidate cold -> hot first, then unwrap)
        const result = await unwrap(
            rpc,
            payer,
            splAta,
            owner,
            mint,
            BigInt(500),
        );

        expect(result).toBeDefined();

        // Check SPL ATA balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(500));

        // Check remaining c-token balance
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(500));
    }, 60_000);

    it('should unwrap c-tokens to SPL ATA (from hot)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create c-token ATA and mint to hot
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(800),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Load to hot first
        const { loadAta } = await import('../../src/v3/actions/load-ata');
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // Verify hot balance
        const hotBalanceBefore = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalanceBefore).toBe(BigInt(800));

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Unwrap partial
        const result = await unwrap(
            rpc,
            payer,
            splAta,
            owner,
            mint,
            BigInt(300),
        );

        expect(result).toBeDefined();

        // Check SPL balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(300));

        // Check remaining c-token balance
        const ctokenBalanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalanceAfter).toBe(BigInt(500));
    }, 60_000);

    it('should unwrap full balance when amount not specified', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(600),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Unwrap all (amount not specified)
        const result = await unwrap(rpc, payer, splAta, owner, mint);

        expect(result).toBeDefined();

        // Check SPL balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(600));

        // c-token should be empty
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(0));
    }, 60_000);

    it('should auto-fetch SPL interface info when not provided', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(400),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Unwrap without providing splInterfaceInfo
        const result = await unwrap(
            rpc,
            payer,
            splAta,
            owner,
            mint,
            BigInt(200),
        );

        expect(result).toBeDefined();

        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(200));
    }, 60_000);

    it('should work with different owners and payers', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const separatePayer = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
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

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Unwrap with separate payer
        const result = await unwrap(
            rpc,
            separatePayer,
            splAta,
            owner,
            mint,
            BigInt(250),
        );

        expect(result).toBeDefined();

        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(250));
    }, 60_000);

    it('should throw error when insufficient balance', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint small amount
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Create destination SPL ATA first (SPL pattern)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Try to unwrap more than available
        await expect(
            unwrap(rpc, payer, splAta, owner, mint, BigInt(1000)),
        ).rejects.toThrow(/Insufficient/);
    }, 60_000);

    it('should throw error when destination does not exist', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Derive but don't create SPL ATA
        const splAta = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        // Try to unwrap to non-existent destination
        await expect(
            unwrap(rpc, payer, splAta, owner, mint, BigInt(50)),
        ).rejects.toThrow(/does not exist/);
    }, 60_000);
});

describe('unwrap Token-2022', () => {
    let rpc: Rpc;
    let payer: Signer;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
    }, 60_000);

    it('should unwrap c-tokens to Token-2022 ATA', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const mintAuthority = Keypair.generate();

        // Create T22 mint
        const mintKeypair = Keypair.generate();
        const { mint: t22Mint } = await createMint(
            rpc,
            payer,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        const tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Create destination T22 ATA first (SPL pattern)
        const t22Ata = await createAssociatedTokenAccount(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        // Unwrap to T22
        const result = await unwrap(
            rpc,
            payer,
            t22Ata,
            owner,
            t22Mint,
            BigInt(500),
        );

        expect(result).toBeDefined();

        // Check T22 ATA balance
        const t22Balance = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22Balance.amount).toBe(BigInt(500));
    }, 90_000);
});
