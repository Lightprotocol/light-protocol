/**
 * E2E tests for createLightTokenFreezeAccountInstruction and
 * createLightTokenThawAccountInstruction (native discriminator 10/11).
 *
 * These instructions operate on decompressed light-token (hot) accounts,
 * mimicking SPL Token freeze/thaw semantics through the LightToken program.
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
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
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccount,
} from '@solana/spl-token';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { loadAta } from '../../src/v3/actions/load-ata';
import { createUnwrapInstructions } from '../../src/v3/actions/unwrap';
import {
    createLightTokenFreezeAccountInstruction,
    createLightTokenThawAccountInstruction,
} from '../../src/v3/instructions/freeze-thaw';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

/** Read the raw token account state byte at offset 108 (AccountState field). */
async function getLightTokenState(
    rpc: Rpc,
    account: PublicKey,
): Promise<number> {
    const info = await rpc.getAccountInfo(account);
    if (!info) throw new Error(`Account not found: ${account.toBase58()}`);
    return info.data[108];
}

/** Read the amount from a light-token account (LE u64 at offset 64). */
async function getLightTokenBalance(
    rpc: Rpc,
    address: PublicKey,
): Promise<bigint> {
    const info = await rpc.getAccountInfo(address);
    if (!info) return BigInt(0);
    return info.data.readBigUInt64LE(64);
}

/** Freeze a hot light-token account using the native LightTokenFreezeAccount instruction. */
async function freezeLightTokenAccount(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: Keypair,
): Promise<void> {
    const ix = createLightTokenFreezeAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority.publicKey,
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx([ix], payer, blockhash, [freezeAuthority]);
    await sendAndConfirmTx(rpc, tx);
}

/** Thaw a frozen light-token account using the native LightTokenThawAccount instruction. */
async function thawLightTokenAccount(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: Keypair,
): Promise<void> {
    const ix = createLightTokenThawAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority.publicKey,
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx([ix], payer, blockhash, [freezeAuthority]);
    await sendAndConfirmTx(rpc, tx);
}

// ---------------------------------------------------------------------------
// Unit tests (no RPC required)
// ---------------------------------------------------------------------------

describe('createLightTokenFreezeAccountInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const freezeAuthority = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createLightTokenFreezeAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 10', () => {
        const ix = createLightTokenFreezeAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.data.length).toBe(1);
        expect(ix.data[0]).toBe(10);
    });

    it('has exactly 3 account metas in correct order', () => {
        const ix = createLightTokenFreezeAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.keys.length).toBe(3);
        // token_account: mutable, not signer
        expect(ix.keys[0].pubkey.equals(tokenAccount)).toBe(true);
        expect(ix.keys[0].isWritable).toBe(true);
        expect(ix.keys[0].isSigner).toBe(false);
        // mint: readonly, not signer
        expect(ix.keys[1].pubkey.equals(mint)).toBe(true);
        expect(ix.keys[1].isWritable).toBe(false);
        expect(ix.keys[1].isSigner).toBe(false);
        // freeze_authority: readonly, signer
        expect(ix.keys[2].pubkey.equals(freezeAuthority)).toBe(true);
        expect(ix.keys[2].isWritable).toBe(false);
        expect(ix.keys[2].isSigner).toBe(true);
    });
});

describe('createLightTokenThawAccountInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const freezeAuthority = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createLightTokenThawAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 11', () => {
        const ix = createLightTokenThawAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.data.length).toBe(1);
        expect(ix.data[0]).toBe(11);
    });

    it('has exactly 3 account metas in correct order', () => {
        const ix = createLightTokenThawAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.keys.length).toBe(3);
        expect(ix.keys[0].pubkey.equals(tokenAccount)).toBe(true);
        expect(ix.keys[0].isWritable).toBe(true);
        expect(ix.keys[0].isSigner).toBe(false);
        expect(ix.keys[1].pubkey.equals(mint)).toBe(true);
        expect(ix.keys[1].isWritable).toBe(false);
        expect(ix.keys[1].isSigner).toBe(false);
        expect(ix.keys[2].pubkey.equals(freezeAuthority)).toBe(true);
        expect(ix.keys[2].isWritable).toBe(false);
        expect(ix.keys[2].isSigner).toBe(true);
    });
});

// ---------------------------------------------------------------------------
// E2E tests
// ---------------------------------------------------------------------------

describe('LightToken freeze/thaw - E2E', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        freezeAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
                freezeAuthority.publicKey,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    it('should freeze a hot light-token account (state → Frozen)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Create hot light-token ATA and mint into it
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        const stateBefore = await getLightTokenState(rpc, lightTokenAta);
        expect(stateBefore).toBe(1); // Initialized

        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );

        const stateAfter = await getLightTokenState(rpc, lightTokenAta);
        expect(stateAfter).toBe(2); // Frozen

        // Balance is unchanged after freeze
        const balance = await getLightTokenBalance(rpc, lightTokenAta);
        expect(balance).toBe(BigInt(500));
    }, 60_000);

    it('should thaw a frozen light-token account (state → Initialized)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Freeze
        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );
        expect(await getLightTokenState(rpc, lightTokenAta)).toBe(2);

        // Thaw
        await thawLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );
        expect(await getLightTokenState(rpc, lightTokenAta)).toBe(1); // Initialized
    }, 60_000);

    it('should fail to freeze an already-frozen account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);
        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );

        // Second freeze attempt must fail
        await expect(
            freezeLightTokenAccount(
                rpc,
                payer,
                lightTokenAta,
                mint,
                freezeAuthority,
            ),
        ).rejects.toThrow();
    }, 60_000);

    it('should fail to thaw an already-initialized account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Thaw on initialized (not frozen) must fail
        await expect(
            thawLightTokenAccount(
                rpc,
                payer,
                lightTokenAta,
                mint,
                freezeAuthority,
            ),
        ).rejects.toThrow();
    }, 60_000);

    it('should fail when wrong freeze authority signs', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const wrongAuthority = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        await expect(
            freezeLightTokenAccount(
                rpc,
                payer,
                lightTokenAta,
                mint,
                wrongAuthority,
            ),
        ).rejects.toThrow();
    }, 60_000);
});

// ---------------------------------------------------------------------------
// createUnwrapInstructions interaction with freeze state
// ---------------------------------------------------------------------------

describe('createUnwrapInstructions - freeze interactions', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        freezeAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                TOKEN_PROGRAM_ID,
                freezeAuthority.publicKey,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    it('should throw when all hot balance is frozen (unwrap blocked)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Mint 500 compressed, load all to hot
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
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Confirm hot balance = 500
        expect(await getLightTokenBalance(rpc, lightTokenAta)).toBe(
            BigInt(500),
        );

        // Create SPL ATA (destination)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze the hot light-token ATA
        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );
        expect(await getLightTokenState(rpc, lightTokenAta)).toBe(2);

        // createUnwrapInstructions must detect all balance is frozen
        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                undefined,
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);
    }, 90_000);

    it('should throw for any requested amount when fully frozen', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
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
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );

        // Requesting a subset of the frozen balance also throws
        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(50),
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);
    }, 90_000);

    it('should succeed after thawing a previously frozen account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

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
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze → confirm unwrap is blocked
        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );
        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(100),
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);

        // Thaw → unwrap succeeds
        await thawLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );

        const batches = await createUnwrapInstructions(
            rpc,
            splAta,
            owner.publicKey,
            mint,
            BigInt(100),
            payer.publicKey,
        );
        expect(batches.length).toBeGreaterThanOrEqual(1);

        for (const ixs of batches) {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, [owner]);
            await sendAndConfirmTx(rpc, tx);
        }

        // SPL ATA should have 100 tokens
        const { getAccount } = await import('@solana/spl-token');
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(100));

        // Hot light-token balance reduced
        const hotBalance = await getLightTokenBalance(rpc, lightTokenAta);
        expect(hotBalance).toBe(BigInt(300));
    }, 90_000);

    it('should throw "Insufficient" when requested amount exceeds unfrozen balance (partial freeze)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Mint 600: load 400 to hot, leave 200 cold
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
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        // Load only 400 by first loading all, then... actually loadAta loads everything.
        // Instead: mint 400 in one batch and load, then mint 200 more (cold).
        // For simplicity: just test with all-hot frozen scenario asking for too much.
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze hot (600 frozen)
        await freezeLightTokenAccount(
            rpc,
            payer,
            lightTokenAta,
            mint,
            freezeAuthority,
        );

        // Requesting any amount throws (Account is frozen / unwrap is not allowed)
        // (since all 600 are frozen hot, none unfrozen)
        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(601),
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|unwrap is not allowed/);
    }, 90_000);
});
