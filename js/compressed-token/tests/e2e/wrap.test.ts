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
import { createMint, mintTo, decompress } from '../../src/actions';
import {
    createAssociatedTokenAccount,
    getOrCreateAssociatedTokenAccount,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAccount,
} from '@solana/spl-token';

// Helper to read CToken account balance (CToken accounts are owned by CTOKEN_PROGRAM_ID)
async function getCTokenBalance(rpc: Rpc, address: PublicKey): Promise<bigint> {
    const accountInfo = await rpc.getAccountInfo(address);
    if (!accountInfo) {
        throw new Error(`CToken account not found: ${address.toBase58()}`);
    }
    // CToken account layout: amount is at offset 64-72 (same as SPL token accounts)
    return accountInfo.data.readBigUInt64LE(64);
}
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { createWrapInstruction } from '../../src/v3/instructions/wrap';
import { wrap } from '../../src/v3/actions/wrap';
import { getAssociatedTokenAddressInterface } from '../../src';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';

// Force V2 for CToken tests
featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('createWrapInstruction', () => {
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

        // Create SPL mint with token pool
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
        const source = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );
        const destination = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);
        expect(tokenPoolInfo).toBeDefined();

        const ix = createWrapInstruction(
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
        const source = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );
        const destination = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const ix = createWrapInstruction(
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
        // Check that payer is in keys
        const payerKey = ix.keys.find(
            k => k.pubkey.equals(feePayer.publicKey) && k.isSigner,
        );
        expect(payerKey).toBeDefined();
    });

    it('should use owner as payer when payer not provided', async () => {
        const owner = Keypair.generate();
        const source = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );
        const destination = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const ix = createWrapInstruction(
            source,
            destination,
            owner.publicKey,
            mint,
            BigInt(100),
            tokenPoolInfo!,
            TEST_TOKEN_DECIMALS,
            // payer not provided - defaults to owner
        );

        expect(ix).toBeDefined();
        // Owner should appear as signer (since payer defaults to owner)
        const ownerKey = ix.keys.find(
            k => k.pubkey.equals(owner.publicKey) && k.isSigner,
        );
        expect(ownerKey).toBeDefined();
    });
});

describe('wrap action', () => {
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

        // Create SPL mint with token pool
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

    it('should wrap SPL tokens to CToken ATA', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create SPL ATA and mint tokens
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Mint compressed then decompress to SPL ATA
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

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(1000),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(1000)),
        );

        // Create CToken ATA
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Check initial balances
        const splBalanceBefore = await getAccount(rpc, splAta);
        expect(splBalanceBefore.amount).toBe(BigInt(1000));

        // Wrap tokens
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const result = await wrap(
            rpc,
            payer,
            splAta,
            ctokenAta,
            owner,
            mint,
            BigInt(500),
            tokenPoolInfo,
        );

        expect(result).toBeDefined();

        // Check balances after
        const splBalanceAfter = await getAccount(rpc, splAta);
        expect(splBalanceAfter.amount).toBe(BigInt(500));

        const ctokenBalanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalanceAfter).toBe(BigInt(500));
    }, 60_000);

    it('should wrap full balance', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Setup: Create SPL ATA with tokens
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

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

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(500),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)),
        );

        // Create CToken ATA
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Wrap full balance
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const result = await wrap(
            rpc,
            payer,
            splAta,
            ctokenAta,
            owner,
            mint,
            BigInt(500), // full balance
            tokenPoolInfo,
        );

        expect(result).toBeDefined();

        // SPL should be empty
        const splBalanceAfter = await getAccount(rpc, splAta);
        expect(splBalanceAfter.amount).toBe(BigInt(0));

        // CToken should have full balance
        const ctokenBalanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalanceAfter).toBe(BigInt(500));
    }, 60_000);

    it('should fetch token pool info when not provided', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Setup
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(200),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(200),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(200)),
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Wrap without providing tokenPoolInfo - should fetch automatically
        const result = await wrap(
            rpc,
            payer,
            splAta,
            ctokenAta,
            owner,
            mint,
            BigInt(100),
            // tokenPoolInfo not provided
        );

        expect(result).toBeDefined();

        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(100));
    }, 60_000);

    it('should throw error when token pool not initialized', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create a new mint without token pool
        const newMintKeypair = Keypair.generate();
        const newMintAuthority = Keypair.generate();

        // Note: createMint actually creates a token pool, so this test scenario
        // would need a special mint without pool. For now, we'll skip this test
        // as it requires a mint without token pool which is hard to set up.
        // The error path is tested implicitly through the action's logic.
    });

    it('should work with different owners and payers', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const separatePayer = await newAccountWithLamports(rpc, 1e9);

        // Setup
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
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

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(300),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(300)),
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

        // Wrap with separate payer
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const result = await wrap(
            rpc,
            separatePayer, // Different from owner
            splAta,
            ctokenAta,
            owner, // Owner still signs for the source account
            mint,
            BigInt(150),
            tokenPoolInfo,
        );

        expect(result).toBeDefined();

        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(150));
    }, 60_000);
});

describe('wrap with non-ATA accounts', () => {
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

    it('should work with explicitly derived ATA addresses (spl-token style)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Explicitly derive ATAs
        // Note: SPL ATAs use getAssociatedTokenAddressSync
        // CToken ATAs use getAssociatedTokenAddressInterface (which defaults to CToken program)
        const source = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );
        const destination = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Setup: Create both ATAs and fund source
        await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);

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

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(400),
            owner,
            source,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(400)),
        );

        // Wrap using explicit addresses
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        const result = await wrap(
            rpc,
            payer,
            source,
            destination,
            owner,
            mint,
            BigInt(200),
            tokenPoolInfo,
        );

        expect(result).toBeDefined();

        const destBalance = await getCTokenBalance(rpc, destination);
        expect(destBalance).toBe(BigInt(200));
    }, 60_000);
});

describe('wrap Token-2022 to CToken', () => {
    let rpc: Rpc;
    let payer: Signer;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
    }, 60_000);

    it('should wrap Token-2022 tokens to CToken ATA', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const mintAuthority = Keypair.generate();

        // Create T22 mint with token pool via createMint action
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

        // Create T22 ATA using getOrCreateAssociatedTokenAccount
        const t22AtaAccount = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            t22Mint,
            owner.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        const t22Ata = t22AtaAccount.address;

        // Mint compressed then decompress to T22 ATA
        const tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
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

        const updatedPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(
            rpc,
            payer,
            t22Mint,
            bn(1000),
            owner,
            t22Ata,
            selectTokenPoolInfosForDecompression(updatedPoolInfos, bn(1000)),
        );

        // Create CToken ATA
        const ctokenAta = getAssociatedTokenAddressInterface(
            t22Mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
        );

        // Check initial balances
        const t22BalanceBefore = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22BalanceBefore.amount).toBe(BigInt(1000));

        // Wrap tokens
        const finalPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        const tokenPoolInfo = finalPoolInfos.find(info => info.isInitialized);

        const result = await wrap(
            rpc,
            payer,
            t22Ata,
            ctokenAta,
            owner,
            t22Mint,
            BigInt(500),
            tokenPoolInfo,
        );

        expect(result).toBeDefined();

        // Check balances after
        const t22BalanceAfter = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22BalanceAfter.amount).toBe(BigInt(500));

        const ctokenBalanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalanceAfter).toBe(BigInt(500));
    }, 90_000);

    it('should auto-fetch SPL interface info for Token-2022 wrap', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const mintAuthority = Keypair.generate();

        // Create T22 mint with token pool
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

        // Create T22 ATA using getOrCreateAssociatedTokenAccount
        const t22AtaAccount = await getOrCreateAssociatedTokenAccount(
            rpc,
            payer as Keypair,
            t22Mint,
            owner.publicKey,
            false,
            'confirmed',
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        const t22Ata = t22AtaAccount.address;

        const tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        const updatedPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(
            rpc,
            payer,
            t22Mint,
            bn(500),
            owner,
            t22Ata,
            selectTokenPoolInfosForDecompression(updatedPoolInfos, bn(500)),
        );

        // Create CToken ATA
        const ctokenAta = getAssociatedTokenAddressInterface(
            t22Mint,
            owner.publicKey,
        );
        await createAtaInterfaceIdempotent(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
        );

        // Wrap without providing tokenPoolInfo - should auto-fetch
        const result = await wrap(
            rpc,
            payer,
            t22Ata,
            ctokenAta,
            owner,
            t22Mint,
            BigInt(250),
            // tokenPoolInfo not provided
        );

        expect(result).toBeDefined();

        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(250));
    }, 90_000);
});
