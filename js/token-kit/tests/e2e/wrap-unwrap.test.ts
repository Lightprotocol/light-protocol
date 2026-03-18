/**
 * E2E tests for wrap (SPL → Light Token) and unwrap (Light Token → SPL).
 *
 * Setup uses V1 createMint (creates SPL mint + SPL interface PDA) with
 * V1 mintTo + decompress to bootstrap SPL tokens.
 *
 * Uses token-kit's createWrapInstruction / createUnwrapInstruction for the
 * actual wrap/unwrap operations, sent via the sendKitInstructions bridge.
 *
 * Requires a running local validator + indexer + prover.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    toKitAddress,
    sendKitInstructions,
    createSplAssociatedTokenAccount,
    getSplTokenBalance,
    getCTokenBalance,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    createMint,
    mintTo,
    decompress,
    getAssociatedTokenAddressInterface,
    createAtaInterfaceIdempotent,
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    type TokenPoolInfo,
    type SplInterfaceInfo as CompressedTokenSplInterfaceInfo,
} from '@lightprotocol/compressed-token';

import {
    selectStateTreeInfo,
    bn,
    type TreeInfo,
} from '@lightprotocol/stateless.js';

import {
    createWrapInstruction,
    createUnwrapInstruction,
    type SplInterfaceInfo,
} from '../../src/index.js';

const DECIMALS = 9;

describe('Wrap / Unwrap e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let mintAddress: ReturnType<typeof toKitAddress>;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
        mintAuthority = await fundAccount(rpc, 1e9);

        // V1 createMint: creates SPL mint (owned by SPL Token Program) + SPL interface PDA
        const result = await createMint(
            rpc,
            payer as any,
            (mintAuthority as any).publicKey,
            DECIMALS,
        );
        mint = result.mint;
        mintAddress = toKitAddress(mint);

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 120_000);

    /** Convert compressed-token SplInterfaceInfo to token-kit SplInterfaceInfo. */
    function toKitSplInterfaceInfo(
        info: CompressedTokenSplInterfaceInfo,
    ): SplInterfaceInfo {
        return {
            poolAddress: toKitAddress(info.splInterfacePda),
            tokenProgram: toKitAddress(info.tokenProgram),
            poolIndex: info.poolIndex,
            bump: info.bump,
            isInitialized: info.isInitialized,
        };
    }

    /**
     * Helper: create an owner with SPL tokens.
     *
     * 1. Mint compressed tokens to owner
     * 2. Create SPL associated token account (standard SPL Token)
     * 3. Decompress to SPL associated token account
     *
     * Returns the owner, SPL associated token account, and SPL interface info.
     */
    async function setupOwnerWithSplTokens(amount: number): Promise<{
        owner: Signer;
        splAta: any;
        splInterfaceInfo: SplInterfaceInfo;
    }> {
        const owner = await fundAccount(rpc, 2e9);

        // Mint compressed tokens
        await mintTo(
            rpc,
            payer as any,
            mint,
            (owner as any).publicKey,
            mintAuthority as any,
            bn(amount),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        // Create SPL associated token account (standard SPL Token — not Token 2022)
        const splAta = await createSplAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            (owner as any).publicKey,
        );

        // Decompress to SPL associated token account
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(
            rpc,
            payer as any,
            mint,
            bn(amount),
            owner as any,
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(amount)),
        );

        // Get SPL interface info
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const compressedTokenSplInfo = tokenPoolInfos.find(
            (info) => info.isInitialized,
        );
        if (!compressedTokenSplInfo) {
            throw new Error('No initialized SPL interface PDA found');
        }

        return {
            owner,
            splAta,
            splInterfaceInfo: toKitSplInterfaceInfo(compressedTokenSplInfo),
        };
    }

    it('wrap: SPL → Light Token associated token account', async () => {
        const { owner, splAta, splInterfaceInfo } =
            await setupOwnerWithSplTokens(1000);

        // Create Light Token associated token account
        await createAtaInterfaceIdempotent(
            rpc,
            payer as any,
            mint,
            (owner as any).publicKey,
        );
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            (owner as any).publicKey,
        );

        // Verify SPL balance before wrap
        expect(await getSplTokenBalance(rpc, splAta)).toBe(1000n);

        // Wrap 500 SPL tokens → Light Token associated token account
        const wrapIx = createWrapInstruction({
            source: toKitAddress(splAta),
            destination: toKitAddress(lightTokenAta),
            owner: toKitAddress((owner as any).publicKey),
            mint: mintAddress,
            amount: 500n,
            splInterfaceInfo,
            decimals: DECIMALS,
            feePayer: toKitAddress((payer as any).publicKey),
        });

        await sendKitInstructions(rpc, [wrapIx], payer, [owner]);

        // Verify: SPL has 500 remaining, Light Token account has 500
        expect(await getSplTokenBalance(rpc, splAta)).toBe(500n);
        expect(await getCTokenBalance(rpc, lightTokenAta)).toBe(500n);
    }, 120_000);

    it('unwrap: Light Token associated token account → SPL', async () => {
        const { owner, splAta, splInterfaceInfo } =
            await setupOwnerWithSplTokens(1000);

        // Create Light Token associated token account
        await createAtaInterfaceIdempotent(
            rpc,
            payer as any,
            mint,
            (owner as any).publicKey,
        );
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            (owner as any).publicKey,
        );

        // Wrap all 1000 → Light Token first
        const wrapIx = createWrapInstruction({
            source: toKitAddress(splAta),
            destination: toKitAddress(lightTokenAta),
            owner: toKitAddress((owner as any).publicKey),
            mint: mintAddress,
            amount: 1000n,
            splInterfaceInfo,
            decimals: DECIMALS,
            feePayer: toKitAddress((payer as any).publicKey),
        });
        await sendKitInstructions(rpc, [wrapIx], payer, [owner]);

        expect(await getSplTokenBalance(rpc, splAta)).toBe(0n);
        expect(await getCTokenBalance(rpc, lightTokenAta)).toBe(1000n);

        // Unwrap 700 Light Token → SPL
        const unwrapIx = createUnwrapInstruction({
            source: toKitAddress(lightTokenAta),
            destination: toKitAddress(splAta),
            owner: toKitAddress((owner as any).publicKey),
            mint: mintAddress,
            amount: 700n,
            splInterfaceInfo,
            decimals: DECIMALS,
            feePayer: toKitAddress((payer as any).publicKey),
        });
        await sendKitInstructions(rpc, [unwrapIx], payer, [owner]);

        expect(await getSplTokenBalance(rpc, splAta)).toBe(700n);
        expect(await getCTokenBalance(rpc, lightTokenAta)).toBe(300n);
    }, 120_000);

    it('round-trip: wrap then unwrap preserves total supply', async () => {
        const { owner, splAta, splInterfaceInfo } =
            await setupOwnerWithSplTokens(2000);

        // Create Light Token associated token account
        await createAtaInterfaceIdempotent(
            rpc,
            payer as any,
            mint,
            (owner as any).publicKey,
        );
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            (owner as any).publicKey,
        );

        const ownerAddr = toKitAddress((owner as any).publicKey);
        const payerAddr = toKitAddress((payer as any).publicKey);
        const splAtaAddr = toKitAddress(splAta);
        const lightTokenAtaAddr = toKitAddress(lightTokenAta);

        // Wrap all 2000 SPL → Light Token
        const wrapIx = createWrapInstruction({
            source: splAtaAddr,
            destination: lightTokenAtaAddr,
            owner: ownerAddr,
            mint: mintAddress,
            amount: 2000n,
            splInterfaceInfo,
            decimals: DECIMALS,
            feePayer: payerAddr,
        });
        await sendKitInstructions(rpc, [wrapIx], payer, [owner]);

        expect(await getSplTokenBalance(rpc, splAta)).toBe(0n);
        expect(await getCTokenBalance(rpc, lightTokenAta)).toBe(2000n);

        // Unwrap all 2000 Light Token → SPL
        const unwrapIx = createUnwrapInstruction({
            source: lightTokenAtaAddr,
            destination: splAtaAddr,
            owner: ownerAddr,
            mint: mintAddress,
            amount: 2000n,
            splInterfaceInfo,
            decimals: DECIMALS,
            feePayer: payerAddr,
        });
        await sendKitInstructions(rpc, [unwrapIx], payer, [owner]);

        expect(await getSplTokenBalance(rpc, splAta)).toBe(2000n);
        expect(await getCTokenBalance(rpc, lightTokenAta)).toBe(0n);
    }, 120_000);
});
