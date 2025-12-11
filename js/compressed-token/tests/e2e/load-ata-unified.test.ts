/**
 * Load ATA - Unified Path (wrap=true)
 *
 * Tests the unified load path which wraps SPL/T22 AND decompresses ctoken-cold.
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
    createAssociatedTokenAccount,
    getOrCreateAssociatedTokenAccount,
    getAccount,
} from '@solana/spl-token';
import { createMint, mintTo, decompress } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

import {
    loadAta as loadAtaUnified,
    createLoadAtaInstructions as createLoadAtaInstructionsUnified,
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

describe('loadAta - Unified Path (wrap=true)', () => {
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

    describe('SPL only', () => {
        it('should wrap SPL balance to ctoken ATA', async () => {
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
                bn(3000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
            await decompress(
                rpc,
                payer,
                mint,
                bn(3000),
                owner,
                splAta,
                selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(3000)),
            );

            const splBalanceBefore = await getAccount(rpc, splAta);
            expect(splBalanceBefore.amount).toBe(BigInt(3000));

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            const signature = await loadAtaUnified(rpc, ctokenAta, owner, mint);

            expect(signature).not.toBeNull();

            const splBalanceAfter = await getAccount(rpc, splAta);
            expect(splBalanceAfter.amount).toBe(BigInt(0));

            const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
            expect(ctokenBalance).toBe(BigInt(3000));
        });
    });

    describe('ctoken-cold only (unified)', () => {
        it('should decompress cold balance via unified path', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(4000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            const signature = await loadAtaUnified(rpc, ctokenAta, owner, mint);

            expect(signature).not.toBeNull();

            const hotBalance = await getCTokenBalance(rpc, ctokenAta);
            expect(hotBalance).toBe(BigInt(4000));
        });
    });

    describe('SPL + ctoken-cold', () => {
        it('should wrap SPL and decompress cold in single load', async () => {
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
                bn(1500),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const splBefore = (await getAccount(rpc, splAta)).amount;
            const coldBefore = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            expect(splBefore).toBe(BigInt(2000));
            expect(coldBefore).toBe(BigInt(1500));

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const splAfter = (await getAccount(rpc, splAta)).amount;
            const coldAfter = await getCompressedBalance(
                rpc,
                owner.publicKey,
                mint,
            );
            const hotBalance = await getCTokenBalance(rpc, ctokenAta);

            expect(splAfter).toBe(BigInt(0));
            expect(coldAfter).toBe(BigInt(0));
            expect(hotBalance).toBe(BigInt(3500));
        });
    });

    describe('ctoken-hot + cold', () => {
        it('should decompress cold to existing hot (no ATA creation)', async () => {
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

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const hotBefore = await getCTokenBalance(rpc, ctokenAta);
            expect(hotBefore).toBe(BigInt(2000));

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

            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const hotAfter = await getCTokenBalance(rpc, ctokenAta);
            expect(hotAfter).toBe(BigInt(3000));
        });
    });

    describe('ctoken-hot + SPL', () => {
        it('should wrap SPL to existing hot ATA', async () => {
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

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const hotBefore = await getCTokenBalance(rpc, ctokenAta);
            expect(hotBefore).toBe(BigInt(1000));

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

            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const hotAfter = await getCTokenBalance(rpc, ctokenAta);
            expect(hotAfter).toBe(BigInt(1500));

            const splAfter = (await getAccount(rpc, splAta)).amount;
            expect(splAfter).toBe(BigInt(0));
        });
    });

    describe('nothing to load', () => {
        it('should throw when no balances exist at all', async () => {
            const owner = Keypair.generate();
            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );

            await expect(
                loadAtaUnified(
                    rpc,
                    ctokenAta,
                    owner as unknown as Signer,
                    mint,
                ),
            ).rejects.toThrow('Token account not found');
        });

        it('should return null when only hot balance exists', async () => {
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

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );
            await loadAtaUnified(rpc, ctokenAta, owner, mint);

            const signature = await loadAtaUnified(rpc, ctokenAta, owner, mint);
            expect(signature).toBeNull();
        });
    });

    describe('createLoadAtaInstructions unified', () => {
        it('should throw when ATA not derived from c-token program', async () => {
            const owner = Keypair.generate();
            const wrongAta = await import('@solana/spl-token').then(m =>
                m.getAssociatedTokenAddressSync(
                    mint,
                    owner.publicKey,
                    false,
                    TOKEN_PROGRAM_ID,
                ),
            );

            await expect(
                createLoadAtaInstructionsUnified(
                    rpc,
                    wrongAta,
                    owner.publicKey,
                    mint,
                    owner.publicKey,
                ),
            ).rejects.toThrow('For wrap=true, ata must be the c-token ATA');
        });

        it('should build instructions for SPL + cold balance', async () => {
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

            const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
                mint,
                owner.publicKey,
            );

            const ixs = await createLoadAtaInstructionsUnified(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
            );

            expect(ixs.length).toBeGreaterThan(1);
        });
    });
});

describe('loadAta - T22 Only', () => {
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

    it('should wrap T22 balance to ctoken ATA', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

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

        t22TokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(
            rpc,
            payer,
            t22Mint,
            bn(2500),
            owner,
            t22Ata,
            selectTokenPoolInfosForDecompression(t22TokenPoolInfos, bn(2500)),
        );

        const t22BalanceBefore = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22BalanceBefore.amount).toBe(BigInt(2500));

        const ctokenAta = getAssociatedTokenAddressInterfaceUnified(
            t22Mint,
            owner.publicKey,
        );
        const signature = await loadAtaUnified(rpc, ctokenAta, owner, t22Mint);

        expect(signature).not.toBeNull();

        const t22BalanceAfter = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22BalanceAfter.amount).toBe(BigInt(0));

        const ctokenBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(ctokenBalance).toBe(BigInt(2500));
    }, 90_000);
});
