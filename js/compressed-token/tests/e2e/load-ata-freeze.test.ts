/**
 * Load ATA - Freeze Interaction Coverage
 *
 * Design: if ANY source (hot or cold) for the ATA is frozen, the entire
 * AccountInterface is treated as frozen. createLoadAtaInstructions and
 * loadAta REJECT (throw) in that case; no instructions are built.
 * getAtaInterface/getAccountInterface show the unified Account as frozen
 * when any source is frozen.
 */
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
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    createAssociatedTokenAccount,
    createFreezeAccountInstruction,
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
    loadAta,
    createLoadAtaInstructions,
} from '../../src/v3/actions/load-ata';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { getAtaInterface } from '../../src/v3/get-account-interface';
import {
    createCTokenFreezeAccountInstruction,
    createCTokenThawAccountInstruction,
} from '../../src/v3/instructions/freeze-thaw';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

async function getCTokenBalance(rpc: Rpc, address: PublicKey): Promise<bigint> {
    const info = await rpc.getAccountInfo(address);
    if (!info) return BigInt(0);
    return info.data.readBigUInt64LE(64);
}

async function getCtokenState(rpc: Rpc, account: PublicKey): Promise<number> {
    const info = await rpc.getAccountInfo(account);
    if (!info) throw new Error(`Account not found: ${account.toBase58()}`);
    return info.data[108];
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

async function freezeCtokenAta(
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
    await sendAndConfirmTx(
        rpc,
        buildAndSignTx([ix], payer, blockhash, [freezeAuthority]),
    );
}

async function thawCtokenAta(
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
    await sendAndConfirmTx(
        rpc,
        buildAndSignTx([ix], payer, blockhash, [freezeAuthority]),
    );
}

async function freezeSplAta(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: Keypair,
    tokenProgram = TOKEN_PROGRAM_ID,
): Promise<void> {
    const ix = createFreezeAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority.publicKey,
        [],
        tokenProgram,
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    await sendAndConfirmTx(
        rpc,
        buildAndSignTx([ix], payer, blockhash, [freezeAuthority]),
    );
}

// ---------------------------------------------------------------------------
// Standard path (wrap=false) - frozen hot ctoken ATA
// ---------------------------------------------------------------------------

describe('loadAta standard (wrap=false) - frozen hot ctoken ATA', () => {
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

    it('getAtaInterface returns parsed.isFrozen true when hot is frozen', async () => {
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
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        const iface = await getAtaInterface(
            rpc,
            ctokenAta,
            owner.publicKey,
            mint,
        );
        expect(iface._anyFrozen).toBe(true);
        expect(iface.parsed.isFrozen).toBe(true);
    }, 90_000);

    it('loadAta throws when hot is frozen and no cold exists', async () => {
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
            bn(500),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);
    }, 90_000);

    it('createLoadAtaInstructions throws when hot is frozen and no cold', async () => {
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
            bn(300),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);
    }, 90_000);

    it('hot frozen account preserves balance when loadAta rejects', async () => {
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
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        const balanceBefore = await getCTokenBalance(rpc, ctokenAta);
        expect(balanceBefore).toBe(BigInt(200));

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        const balanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(balanceAfter).toBe(BigInt(200));
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);
    }, 90_000);

    it('thaw restores normal load behavior', async () => {
        // Freeze hot → loadAta null → thaw → mint more cold → loadAta succeeds
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
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        // Thaw then mint more cold
        await thawCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
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

        const result = await loadAta(rpc, ctokenAta, owner, mint, payer);
        expect(result).not.toBeNull();

        const balance = await getCTokenBalance(rpc, ctokenAta);
        expect(balance).toBe(BigInt(600)); // 400 original + 200 new cold
    }, 90_000);

    it('hot frozen + cold unfrozen → SDK rejects entirely (no instructions)', async () => {
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
            bn(300),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
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

        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(300));
        expect(await getCompressedBalance(rpc, owner.publicKey, mint)).toBe(
            BigInt(200),
        );
    }, 90_000);
});

// ---------------------------------------------------------------------------
// Unified path (wrap=true) - frozen SPL source
// ---------------------------------------------------------------------------

describe('loadAta unified (wrap=true) - frozen SPL source', () => {
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

    it('loadAta throws when SPL source is frozen and no cold exists (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );
        // Decompress some compressed tokens to the SPL ATA
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
        const splBefore = await getAccount(rpc, splAta);
        expect(splBefore.amount).toBe(BigInt(500));

        // Freeze the SPL ATA
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await expect(
            loadAta(
                rpc,
                ctokenAta,
                owner,
                mint,
                payer,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        const splAfter = await getAccount(rpc, splAta);
        expect(splAfter.amount).toBe(BigInt(500));
        expect(splAfter.isFrozen).toBe(true);
    }, 90_000);

    it('createLoadAtaInstructions throws when SPL frozen, no cold (wrap=true)', async () => {
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
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);
    }, 90_000);

    it('SPL frozen + cold unfrozen → SDK rejects entirely (wrap=true)', async () => {
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

        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer, undefined, undefined, true),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        const splAfter = await getAccount(rpc, splAta);
        expect(splAfter.amount).toBe(BigInt(500));
        expect(splAfter.isFrozen).toBe(true);
        expect(await getCompressedBalance(rpc, owner.publicKey, mint)).toBe(
            BigInt(400),
        );
    }, 90_000);
});

// ---------------------------------------------------------------------------
// Unified path (wrap=true) - frozen T22 source
// ---------------------------------------------------------------------------

describe('loadAta unified (wrap=true) - frozen T22 source', () => {
    let rpc: Rpc;
    let payer: Signer;
    let t22Mint: PublicKey;
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

        t22Mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                TOKEN_2022_PROGRAM_ID,
                freezeAuthority.publicKey,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
    }, 60_000);

    it('loadAta throws when T22 source is frozen and no cold exists (wrap=true)', async () => {
        const { getOrCreateAssociatedTokenAccount } = await import(
            '@solana/spl-token'
        );
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
            mintAuthority,
            bn(500),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(
            rpc,
            payer,
            t22Mint,
            bn(500),
            owner,
            t22Ata,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)),
        );

        await freezeSplAta(
            rpc,
            payer,
            t22Ata,
            t22Mint,
            freezeAuthority,
            TOKEN_2022_PROGRAM_ID,
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            t22Mint,
            owner.publicKey,
        );
        await expect(
            loadAta(
                rpc,
                ctokenAta,
                owner,
                t22Mint,
                payer,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        const t22After = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22After.isFrozen).toBe(true);
    }, 90_000);

    it('T22 frozen + cold unfrozen → SDK rejects entirely (wrap=true)', async () => {
        const { getOrCreateAssociatedTokenAccount } = await import(
            '@solana/spl-token'
        );
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
            mintAuthority,
            bn(600),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(
            rpc,
            payer,
            t22Mint,
            bn(600),
            owner,
            t22Ata,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(600)),
        );
        await mintTo(
            rpc,
            payer,
            t22Mint,
            owner.publicKey,
            mintAuthority,
            bn(300),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        await freezeSplAta(
            rpc,
            payer,
            t22Ata,
            t22Mint,
            freezeAuthority,
            TOKEN_2022_PROGRAM_ID,
        );

        const ctokenAta = getAssociatedTokenAddressInterface(
            t22Mint,
            owner.publicKey,
        );
        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                t22Mint,
                payer.publicKey,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        await expect(
            loadAta(
                rpc,
                ctokenAta,
                owner,
                t22Mint,
                payer,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        const t22After = await getAccount(
            rpc,
            t22Ata,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(t22After.amount).toBe(BigInt(600));
        expect(t22After.isFrozen).toBe(true);
        expect(await getCompressedBalance(rpc, owner.publicKey, t22Mint)).toBe(
            BigInt(300),
        );
    }, 90_000);
});

// ---------------------------------------------------------------------------
// Combined freeze scenarios (wrap=true)
// ---------------------------------------------------------------------------

describe('loadAta unified (wrap=true) - combined freeze scenarios', () => {
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

    it('hot ctoken frozen + SPL unfrozen → SDK rejects entirely (wrap=true)', async () => {
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
            bn(300),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

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
        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(500));

        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(500));
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(300));
    }, 90_000);

    it('SPL frozen + cold unfrozen → SDK rejects entirely (wrap=true)', async () => {
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
            splAta,
            selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(400)),
        );
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(250),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );

        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );
        await expect(
            loadAta(
                rpc,
                ctokenAta,
                owner,
                mint,
                payer,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(400));
        expect((await getAccount(rpc, splAta)).isFrozen).toBe(true);
        expect(await getCompressedBalance(rpc, owner.publicKey, mint)).toBe(
            BigInt(250),
        );
    }, 90_000);

    it('all sources frozen → SDK rejects (wrap=true)', async () => {
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
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

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
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        await expect(
            createLoadAtaInstructions(
                rpc,
                ctokenAta,
                owner.publicKey,
                mint,
                payer.publicKey,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);

        await expect(
            loadAta(
                rpc,
                ctokenAta,
                owner,
                mint,
                payer,
                undefined,
                undefined,
                true,
            ),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);
    }, 90_000);

    it('all sources frozen → loadAta throws (wrap=false)', async () => {
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
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        await expect(
            loadAta(rpc, ctokenAta, owner, mint, payer),
        ).rejects.toThrow(/Account is frozen|load is not allowed/);
    }, 90_000);

    it('loadAta targeting SPL ATA uses SPL+cold view only (ctoken hot not in that view)', async () => {
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
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

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
        const splAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            owner.publicKey,
        );

        const result = await loadAta(rpc, splAta, owner, mint, payer);
        expect(result).not.toBeNull();

        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(300));
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(200));
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);
    }, 90_000);
});
