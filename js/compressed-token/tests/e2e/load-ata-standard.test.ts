/**
 * Load ATA - Standard Path (wrap=false)
 *
 * Tests the standard load path which only decompresses ctoken-cold.
 * SPL/T22 balances are NOT wrapped in this mode.
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
    LIGHT_TOKEN_PROGRAM_ID,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    createAssociatedTokenAccount,
    getAccount,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import { createMint, mintTo, decompress } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

import {
    loadAta,
    createLoadAtaInstructions,
} from '../../src/v3/actions/load-ata';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';

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

describe('loadAta - Standard Path (wrap=false)', () => {
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

    describe('ctoken-cold only', () => {
        it('should decompress cold balance to hot ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(5000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const coldBefore = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(coldBefore).toBe(BigInt(5000));

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ata, owner, mint);

            expect(signature).not.toBeNull();

            const hotBalance = await getCTokenBalance(rpc, ata);
            expect(hotBalance).toBe(BigInt(5000));

            const coldAfter = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(coldAfter).toBe(BigInt(0));
        });

        it('should create ATA if it does not exist', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

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

            const ataBefore = await rpc.getAccountInfo(ata);
            expect(ataBefore).toBeNull();

            await loadAta(rpc, ata, owner, mint);

            const ataAfter = await rpc.getAccountInfo(ata);
            expect(ataAfter).not.toBeNull();
        });
    });

    describe('ctoken-hot exists', () => {
        it('should return null when no cold balance (nothing to load)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
            );

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            const signature = await loadAta(rpc, ata, owner, mint);
            expect(signature).toBeNull();
        });

        it('should decompress cold to existing hot ATA (additive)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            await loadAta(rpc, ata, owner, mint);

            const hotBefore = await getCTokenBalance(rpc, ata);
            expect(hotBefore).toBe(BigInt(3000));

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

            await loadAta(rpc, ata, owner, mint);

            const hotAfter = await getCTokenBalance(rpc, ata);
            expect(hotAfter).toBe(BigInt(5000));
        });
    });

    describe('SPL/T22 balances (wrap=false)', () => {
        it('should NOT wrap SPL balance when wrap=false', async () => {
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
                bn(500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const splBalanceBefore = await getAccount(rpc, splAta);
            expect(splBalanceBefore.amount).toBe(BigInt(2000));

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const signature = await loadAta(rpc, ctokenAta, owner, mint);

            expect(signature).not.toBeNull();

            const splBalanceAfter = await getAccount(rpc, splAta);
            expect(splBalanceAfter.amount).toBe(BigInt(2000));

            const hotBalance = await getCTokenBalance(rpc, ctokenAta);
            expect(hotBalance).toBe(BigInt(500));
        });
    });

    describe('createLoadAtaInstructions', () => {
        it('should return empty when no accounts exist', async () => {
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

        it('should return empty when hot exists but no cold', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                mint,
                owner.publicKey,
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
                payer.publicKey,
            );

            expect(batches.length).toBe(0);
        });

        it('should build instructions for cold balance', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

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

            expect(batches.length).toBeGreaterThan(0);
        });
    });

});
