/**
 * V3 Interface V1/V2 Test Suite
 *
 * Tests that v3 interface rejects V1 accounts with meaningful errors.
 * V1 users must use main SDK actions (transfer, decompress, merge) to migrate.
 */
import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    TreeType,
    featureFlags,
    VERSION,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import {
    decompressInterface,
    getAtaInterface,
    getAssociatedTokenAddressInterface,
    transferInterface,
} from '../../src/v3';
import { createLoadAtaInstructions, loadAta } from '../../src/index';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('v3-interface-v1-rejection', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintAuthority: Keypair;
    let mint: PublicKey;
    let treeInfos: TreeInfo[];
    let v1TreeInfo: TreeInfo;
    let v2TreeInfo: TreeInfo;
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

        treeInfos = await rpc.getStateTreeInfos();
        v1TreeInfo = selectStateTreeInfo(treeInfos, TreeType.StateV1);
        v2TreeInfo = selectStateTreeInfo(treeInfos, TreeType.StateV2);
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 120_000);

    describe('decompressInterface', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('rejects V1 accounts with meaningful error', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await expect(
                decompressInterface(rpc, payer, owner, mint, bn(500)),
            ).rejects.toThrow(
                'v3 interface does not support V1 compressed accounts',
            );
        });

        it('rejects mixed V1+V2 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await expect(
                decompressInterface(rpc, payer, owner, mint, bn(200)),
            ).rejects.toThrow(
                'v3 interface does not support V1 compressed accounts',
            );
        });

        it('succeeds with only V2 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const sig = await decompressInterface(
                rpc,
                payer,
                owner,
                mint,
                bn(500),
            );
            expect(sig).toBeDefined();
            expect(sig).not.toBeNull();
        });
    });

    describe('loadAta / createLoadAtaInstructions', () => {
        let owner: Signer;
        let ctokenAta: PublicKey;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
            ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
        });

        it('rejects V1 accounts with meaningful error', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await expect(
                createLoadAtaInstructions(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    mint,
                    payer.publicKey,
                ),
            ).rejects.toThrow(
                'v3 interface does not support V1 compressed accounts',
            );
        });

        it('rejects mixed V1+V2 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(300),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            await expect(
                createLoadAtaInstructions(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    mint,
                    payer.publicKey,
                ),
            ).rejects.toThrow(
                'v3 interface does not support V1 compressed accounts',
            );
        });

        it('succeeds with only V2 accounts', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(500),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const sig = await loadAta(rpc, ctokenAta, owner, mint, payer);
            expect(sig === null || typeof sig === 'string').toBe(true);
        });
    });

    describe('getAtaInterface', () => {
        let owner: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
        });

        it('discovers V2 accounts correctly', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v2TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ataInfo = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
            );

            expect(ataInfo.parsed.amount).toBeGreaterThanOrEqual(BigInt(1000));
        });

        it('discovers V1 accounts (read-only, no error)', async () => {
            // getAtaInterface is read-only, so it can discover V1 accounts
            // The error happens when trying to USE them in operations
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const ataInfo = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
            );

            // Should discover the V1 balance
            expect(ataInfo.isCold).toBe(true);
        });
    });

    describe('transferInterface', () => {
        let owner: Signer;
        let recipient: Signer;

        beforeEach(async () => {
            owner = await newAccountWithLamports(rpc, 1e9);
            recipient = await newAccountWithLamports(rpc, 1e9);
        });

        it('rejects V1 accounts with meaningful error', async () => {
            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                v1TreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const sourceAta = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );

            // transferInterface(rpc, payer, source, mint, recipientWallet, owner, amount)
            await expect(
                transferInterface(
                    rpc,
                    payer,
                    sourceAta,
                    mint,
                    recipient.publicKey,
                    owner,
                    BigInt(500),
                ),
            ).rejects.toThrow(
                'v3 interface does not support V1 compressed accounts',
            );
        });

        // Note: V2 success case is covered by existing v3 interface tests.
        // The V1 rejection test validates our assertV2Only check works.
    });
});
