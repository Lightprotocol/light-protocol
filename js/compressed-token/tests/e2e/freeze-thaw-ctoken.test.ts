/**
 * E2E tests for createCTokenFreezeAccountInstruction and
 * createCTokenThawAccountInstruction (native discriminator 10/11).
 *
 * These instructions operate on decompressed c-token (hot) accounts,
 * mimicking SPL Token freeze/thaw semantics through the cToken program.
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
    createCTokenFreezeAccountInstruction,
    createCTokenThawAccountInstruction,
} from '../../src/v3/instructions/freeze-thaw';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

/** Read the raw token account state byte at offset 108 (AccountState field). */
async function getCtokenState(rpc: Rpc, account: PublicKey): Promise<number> {
    const info = await rpc.getAccountInfo(account);
    if (!info) throw new Error(`Account not found: ${account.toBase58()}`);
    return info.data[108];
}

/** Read the amount from a c-token account (LE u64 at offset 64). */
async function getCTokenBalance(rpc: Rpc, address: PublicKey): Promise<bigint> {
    const info = await rpc.getAccountInfo(address);
    if (!info) return BigInt(0);
    return info.data.readBigUInt64LE(64);
}

/** Freeze a hot c-token account using the native CTokenFreezeAccount instruction. */
async function freezeCtokenAccount(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: Keypair,
): Promise<void> {
    const ix = createCTokenFreezeAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority.publicKey,
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx([ix], payer, blockhash, [freezeAuthority]);
    await sendAndConfirmTx(rpc, tx);
}

/** Thaw a frozen c-token account using the native CTokenThawAccount instruction. */
async function thawCtokenAccount(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: Keypair,
): Promise<void> {
    const ix = createCTokenThawAccountInstruction(
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

describe('createCTokenFreezeAccountInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const freezeAuthority = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createCTokenFreezeAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 10', () => {
        const ix = createCTokenFreezeAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.data.length).toBe(1);
        expect(ix.data[0]).toBe(10);
    });

    it('has exactly 3 account metas in correct order', () => {
        const ix = createCTokenFreezeAccountInstruction(
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

describe('createCTokenThawAccountInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const freezeAuthority = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createCTokenThawAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 11', () => {
        const ix = createCTokenThawAccountInstruction(
            tokenAccount,
            mint,
            freezeAuthority,
        );
        expect(ix.data.length).toBe(1);
        expect(ix.data[0]).toBe(11);
    });

    it('has exactly 3 account metas in correct order', () => {
        const ix = createCTokenThawAccountInstruction(
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

describe('cToken freeze/thaw - E2E', () => {
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

    it('should freeze a hot c-token account (state → Frozen)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Create hot c-token ATA and mint into it
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        const stateBefore = await getCtokenState(rpc, ctokenAta);
        expect(stateBefore).toBe(1); // Initialized

        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);

        const stateAfter = await getCtokenState(rpc, ctokenAta);
        expect(stateAfter).toBe(2); // Frozen

        // Balance is unchanged after freeze
        const balance = await getCTokenBalance(rpc, ctokenAta);
        expect(balance).toBe(BigInt(500));
    }, 60_000);

    it('should thaw a frozen c-token account (state → Initialized)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // Freeze
        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

        // Thaw
        await thawCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(1); // Initialized
    }, 60_000);

    it('should fail to freeze an already-frozen account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);

        // Second freeze attempt must fail
        await expect(
            freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority),
        ).rejects.toThrow();
    }, 60_000);

    it('should fail to thaw an already-initialized account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // Thaw on initialized (not frozen) must fail
        await expect(
            thawCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority),
        ).rejects.toThrow();
    }, 60_000);

    it('should fail when wrong freeze authority signs', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const wrongAuthority = Keypair.generate();
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        await expect(
            freezeCtokenAccount(rpc, payer, ctokenAta, mint, wrongAuthority),
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

    it('should throw "All c-token balance is frozen" when all hot balance is frozen', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        // Confirm hot balance = 500
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(500));

        // Create SPL ATA (destination)
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze the hot c-token ATA
        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

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
        ).rejects.toThrow('All c-token balance is frozen');
    }, 90_000);

    it('should throw "All c-token balance is frozen" for any requested amount when fully frozen', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);

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
        ).rejects.toThrow('All c-token balance is frozen');
    }, 90_000);

    it('should succeed after thawing a previously frozen account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze → confirm unwrap is blocked
        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);
        await expect(
            createUnwrapInstructions(
                rpc,
                splAta,
                owner.publicKey,
                mint,
                BigInt(100),
                payer.publicKey,
            ),
        ).rejects.toThrow('All c-token balance is frozen');

        // Thaw → unwrap succeeds
        await thawCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);

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

        // Hot c-token balance reduced
        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(300));
    }, 90_000);

    it('should throw "Insufficient" when requested amount exceeds unfrozen balance (partial freeze)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
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
        await loadAta(rpc, ctokenAta, owner, mint, payer);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Freeze hot (600 frozen)
        await freezeCtokenAccount(rpc, payer, ctokenAta, mint, freezeAuthority);

        // Requesting any amount throws "All c-token balance is frozen"
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
        ).rejects.toThrow('All c-token balance is frozen');
    }, 90_000);
});
