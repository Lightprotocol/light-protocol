/**
 * Load ATA - Combined Tests
 *
 * Tests combined scenarios, export path verification, payer handling, and idempotency.
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
import { createAssociatedTokenAccount, getAccount } from '@solana/spl-token';
import { createMint, mintTo, decompress } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

import { loadAta as loadAtaStandard } from '../../src/v3/actions/load-ata';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';

import {
    loadAta as loadAtaUnified,
    getAssociatedTokenAddressInterface as getAssociatedTokenAddressInterfaceUnified,
} from '../../src/v3/unified';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

async function getCTokenBalance(rpc: Rpc, address: PublicKey): Promise<bigint> {
    const accountInfo = await rpc.getAccountInfo(address);
    if (!accountInfo) return BigInt(0);
    return accountInfo.data.readBigUInt64LE(64);
}

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

describe('loadAta - All Sources Combined', () => {
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

    it('should load SPL + ctoken-cold all at once', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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
            bn(2000),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(2000),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(2000)),
        );

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

        const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
            mint,
            owner.publicKey,
        );
        await loadAtaUnified(rpc, ctokenAta, owner, mint);

        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(3000));

        const splBalance = (await getAccount(rpc, splAta)).amount;
        expect(splBalance).toBe(BigInt(0));

        const coldBalance = await getCompressedBalance(
            rpc,
            owner.publicKey,
            mint,
        );
        expect(coldBalance).toBe(BigInt(0));
    });
});

describe('loadAta - Export Path Verification', () => {
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

    it('standard export with wrap=true behaves like unified', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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
            bn(1500),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(1500),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(1500)),
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        const signature = await loadAtaStandard(
            rpc,
            ctokenAta,
            owner,
            mint,
            undefined,
            undefined,
            undefined,
            true, // wrap=true
        );

        expect(signature).not.toBeNull();

        const splBalance = (await getAccount(rpc, splAta)).amount;
        expect(splBalance).toBe(BigInt(0));

        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(1500));
    });

    it('unified export always wraps (wrap=true default)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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
            bn(1200),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer,
            mint,
            bn(1200),
            owner,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(1200)),
        );

        const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
            mint,
            owner.publicKey,
        );
        await loadAtaUnified(rpc, ctokenAta, owner, mint);

        const splBalance = (await getAccount(rpc, splAta)).amount;
        expect(splBalance).toBe(BigInt(0));

        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(1200));
    });
});

describe('loadAta - Payer Handling', () => {
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

    it('should use separate payer when provided', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const separatePayer = await newAccountWithLamports(rpc, 1e9);

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

        const ata = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        const signature = await loadAtaStandard(
            rpc,
            ata,
            owner,
            mint,
            separatePayer,
        );

        expect(signature).not.toBeNull();

        const hotBalance = await getCTokenBalance(rpc, ata);
        expect(hotBalance).toBe(BigInt(1000));
    });

    it('should default payer to owner when not provided', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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

        const ata = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        const signature = await loadAtaStandard(rpc, ata, owner, mint);

        expect(signature).not.toBeNull();

        const hotBalance = await getCTokenBalance(rpc, ata);
        expect(hotBalance).toBe(BigInt(800));
    });
});

describe('loadAta - Idempotency', () => {
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

    it('should be idempotent - multiple loads do not fail', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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

        const ata = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        const sig1 = await loadAtaStandard(rpc, ata, owner, mint);
        expect(sig1).not.toBeNull();

        const sig2 = await loadAtaStandard(rpc, ata, owner, mint);
        expect(sig2).toBeNull();

        const sig3 = await loadAtaStandard(rpc, ata, owner, mint);
        expect(sig3).toBeNull();

        const hotBalance = await getCTokenBalance(rpc, ata);
        expect(hotBalance).toBe(BigInt(2000));
    });
});
