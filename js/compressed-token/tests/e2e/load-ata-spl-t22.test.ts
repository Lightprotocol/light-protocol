/**
 * Load ATA - SPL/T22 Decompression (wrap=false)
 *
 * Tests decompressing compressed tokens to SPL/T22 ATAs via token pools.
 * This is the standard path where compressed tokens are loaded into
 * SPL or T22 ATAs rather than c-token ATAs.
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    getAccount,
    createAssociatedTokenAccount,
    getOrCreateAssociatedTokenAccount,
} from '@solana/spl-token';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAtaProgramId } from '../../src/v3/ata-utils';

import {
    loadAta,
    createLoadAtaInstructions,
} from '../../src/v3/actions/load-ata';
import { checkAtaAddress } from '../../src/v3/ata-utils';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

async function getCompressedBalance(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
): Promise<bigint> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, { mint });
    return result.items.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );
}

describe('checkAtaAddress', () => {
    it('should validate c-token ATA', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner);
        const result = checkAtaAddress(ctokenAta, mint, owner);
        expect(result.valid).toBe(true);
        expect(result.type).toBe('ctoken');
    });

    it('should validate SPL ATA', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        const splAta = getAssociatedTokenAddressSync(
            mint,
            owner,
            false,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );
        const result = checkAtaAddress(splAta, mint, owner);
        expect(result.valid).toBe(true);
        expect(result.type).toBe('spl');
    });

    it('should validate Token-2022 ATA', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        const t22Ata = getAssociatedTokenAddressSync(
            mint,
            owner,
            false,
            TOKEN_2022_PROGRAM_ID,
            getAtaProgramId(TOKEN_2022_PROGRAM_ID),
        );
        const result = checkAtaAddress(t22Ata, mint, owner);
        expect(result.valid).toBe(true);
        expect(result.type).toBe('token2022');
    });

    it('should throw on invalid ATA address', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const wrongAta = Keypair.generate().publicKey;

        expect(() => checkAtaAddress(wrongAta, mint, owner)).toThrow(
            'ATA address does not match any valid derivation',
        );
    });

    it('should use hot path when programId is provided', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        const splAta = getAssociatedTokenAddressSync(
            mint,
            owner,
            false,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );
        const result = checkAtaAddress(splAta, mint, owner, TOKEN_PROGRAM_ID);
        expect(result.valid).toBe(true);
        expect(result.type).toBe('spl');
    });
});

describe('loadAta - Decompress to SPL ATA', () => {
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

    it('should decompress compressed tokens to SPL ATA via token pool', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(3000),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Verify compressed balance
        const coldBefore = await getCompressedBalance(
            rpc,
            owner.publicKey,
            mint,
        );
        expect(coldBefore).toBe(BigInt(3000));

        // Create SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Load to SPL ATA (not c-token ATA!)
        const signature = await loadAta(rpc, splAta, owner, mint, payer);

        expect(signature).not.toBeNull();

        // Verify SPL ATA has the balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(3000));

        // Verify compressed balance is gone
        const coldAfter = await getCompressedBalance(
            rpc,
            owner.publicKey,
            mint,
        );
        expect(coldAfter).toBe(BigInt(0));
    });

    it('should create SPL ATA if needed when decompressing', async () => {
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

        // Derive SPL ATA (don't create it)
        const splAta = getAssociatedTokenAddressSync(
            mint,
            owner.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );

        // Verify SPL ATA doesn't exist
        const ataBefore = await rpc.getAccountInfo(splAta);
        expect(ataBefore).toBeNull();

        // Load to SPL ATA - should auto-create
        const signature = await loadAta(rpc, splAta, owner, mint, payer);

        expect(signature).not.toBeNull();

        // Verify SPL ATA was created with balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(2000));
    });

    it('should add to existing SPL ATA balance', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create SPL ATA with initial balance
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Mint and decompress first batch directly to SPL
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
        await loadAta(rpc, splAta, owner, mint, payer);

        const balanceAfterFirst = await getAccount(rpc, splAta);
        expect(balanceAfterFirst.amount).toBe(BigInt(1000));

        // Mint more compressed tokens
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

        // Load second batch
        await loadAta(rpc, splAta, owner, mint, payer);

        // Verify total balance
        const balanceAfterSecond = await getAccount(rpc, splAta);
        expect(balanceAfterSecond.amount).toBe(BigInt(1500));
    });
});

describe('loadAta - Decompress to T22 ATA', () => {
    let rpc: Rpc;
    let payer: Signer;
    let t22Mint: PublicKey;
    let t22MintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let t22TokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        t22MintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        const result = await createMint(
            rpc,
            payer,
            t22MintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        t22Mint = result.mint;
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        t22TokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
    }, 60_000);

    it('should decompress compressed tokens to T22 ATA via token pool', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            t22MintAuthority,
            bn(2500),
            stateTreeInfo,
            selectTokenPoolInfo(t22TokenPoolInfos),
        );

        // Verify compressed balance
        const coldBefore = await getCompressedBalance(
            rpc,
            owner.publicKey,
            t22Mint,
        );
        expect(coldBefore).toBe(BigInt(2500));

        // Create T22 ATA
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

        // Load to T22 ATA
        const signature = await loadAta(rpc, t22Ata, owner, t22Mint, payer);

        expect(signature).not.toBeNull();

        // Verify T22 ATA has the balance
        const t22Balance = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22Balance.amount).toBe(BigInt(2500));

        // Verify compressed balance is gone
        const coldAfter = await getCompressedBalance(
            rpc,
            owner.publicKey,
            t22Mint,
        );
        expect(coldAfter).toBe(BigInt(0));
    }, 90_000);

    it('should add to existing T22 ATA balance', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Create T22 ATA
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

        // Mint and load first batch
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            t22MintAuthority,
            bn(1000),
            stateTreeInfo,
            selectTokenPoolInfo(t22TokenPoolInfos),
        );
        await loadAta(rpc, t22Ata, owner, t22Mint, payer);

        const balanceAfterFirst = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(balanceAfterFirst.amount).toBe(BigInt(1000));

        // Mint and load second batch
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            t22MintAuthority,
            bn(800),
            stateTreeInfo,
            selectTokenPoolInfo(t22TokenPoolInfos),
        );
        await loadAta(rpc, t22Ata, owner, t22Mint, payer);

        const balanceAfterSecond = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(balanceAfterSecond.amount).toBe(BigInt(1800));
    }, 90_000);
});

describe('loadAta - Standard vs Unified Distinction', () => {
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

    it('wrap=false with SPL ATA decompresses to SPL, not c-token', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Mint compressed tokens
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

        // Create SPL ATA
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        // Load to SPL ATA with wrap=false (default)
        await loadAta(rpc, splAta, owner, mint, payer);

        // SPL ATA should have balance
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(1500));

        // c-token ATA should NOT exist (we didn't create it)
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const ctokenInfo = await rpc.getAccountInfo(ctokenAta);
        expect(ctokenInfo).toBeNull();
    });

    it('wrap=false with c-token ATA decompresses to c-token', async () => {
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

        // Derive c-token ATA
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Load to c-token ATA with wrap=false
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // c-token ATA should have balance
        const ctokenInfo = await rpc.getAccountInfo(ctokenAta);
        expect(ctokenInfo).not.toBeNull();
        const ctokenBalance = ctokenInfo!.data.readBigUInt64LE(64);
        expect(ctokenBalance).toBe(BigInt(2000));
    });
});
