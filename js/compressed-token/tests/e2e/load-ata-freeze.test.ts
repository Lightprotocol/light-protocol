/**
 * Load ATA - Freeze Interaction Coverage
 *
 * Tests all combinations of frozen sources (hot ctoken, SPL, T22, cold) with
 * createLoadAtaInstructions and loadAta (both wrap=false and wrap=true paths).
 *
 * Design invariants (from _buildLoadBatches source filtering):
 *   - Frozen hot ctoken ATA  → treated as absent; needsAtaCreation=true
 *   - Frozen SPL source      → excluded; splBalance=0
 *   - Frozen T22 source      → excluded; t22Balance=0
 *   - Frozen cold sources    → excluded from allCompressedAccounts
 *   - If splBalance+t22Balance+coldBalance==0 → returns []
 *
 * Any scenario where all loadable balances are zero must return [] /null.
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
    await sendAndConfirmTx(rpc, buildAndSignTx([ix], payer, blockhash, [freezeAuthority]));
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
    await sendAndConfirmTx(rpc, buildAndSignTx([ix], payer, blockhash, [freezeAuthority]));
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
    await sendAndConfirmTx(rpc, buildAndSignTx([ix], payer, blockhash, [freezeAuthority]));
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

    it('returns null when hot is frozen and no cold exists', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Create hot, load cold, freeze hot – leaves zero cold remaining
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(500), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        // No cold, hot is frozen → nothing to load
        const result = await loadAta(rpc, ctokenAta, owner, mint, payer);
        expect(result).toBeNull();
    }, 90_000);

    it('createLoadAtaInstructions returns [] when hot is frozen and no cold', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey,
        );
        expect(batches.length).toBe(0);
    }, 90_000);

    it('hot frozen account preserves balance after failed load attempt', async () => {
        // If hot is frozen and no cold, loadAta returns null without touching the account.
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(200), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        const balanceBefore = await getCTokenBalance(rpc, ctokenAta);
        expect(balanceBefore).toBe(BigInt(200));

        await loadAta(rpc, ctokenAta, owner, mint, payer); // no-op

        const balanceAfter = await getCTokenBalance(rpc, ctokenAta);
        expect(balanceAfter).toBe(BigInt(200)); // unchanged
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2); // still frozen
    }, 90_000);

    it('thaw restores normal load behavior', async () => {
        // Freeze hot → loadAta null → thaw → mint more cold → loadAta succeeds
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(400), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        // Nothing to load while frozen
        expect(await loadAta(rpc, ctokenAta, owner, mint, payer)).toBeNull();

        // Thaw then mint more cold
        await thawCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(200), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        const result = await loadAta(rpc, ctokenAta, owner, mint, payer);
        expect(result).not.toBeNull();

        const balance = await getCTokenBalance(rpc, ctokenAta);
        expect(balance).toBe(BigInt(600)); // 400 original + 200 new cold
    }, 90_000);

    it('hot frozen + cold unfrozen → cold is filtered from load (instructions built, decompress attempted)', async () => {
        // Load partial (300), keep 200 cold, then freeze hot.
        // The SDK sees cold=200 (unfrozen) and tries to build load instructions.
        // The instructions ARE built (cold is not frozen), but execution into a
        // frozen hot ATA will fail on-chain because Transfer2 enforces frozen state.
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Mint 500 cold, load 300 to hot (leaves 200 cold)
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(200), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        // Freeze the hot ATA (200 cold still unfrozen in Merkle tree)
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

        // The SDK builds load instructions (cold=200 is unfrozen → included)
        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey,
        );
        // Instructions ARE generated for the unfrozen cold (200)
        expect(batches.length).toBeGreaterThan(0);

        // Executing them fails: Transfer2 decompresses into frozen hot ATA → on-chain error
        // (The ctoken program / pinocchio enforces frozen state on the destination)
        const batch = batches[0];
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(batch, payer, blockhash, [owner]);
        await expect(sendAndConfirmTx(rpc, tx)).rejects.toThrow();

        // Hot balance is unchanged (freeze blocked the decompress)
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(300));
        // Cold 200 is still in the Merkle tree
        expect(await getCompressedBalance(rpc, owner.publicKey, mint)).toBe(BigInt(200));
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

    it('returns null when SPL source is frozen and no cold exists (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        // Decompress some compressed tokens to the SPL ATA
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(500), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(500), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)));
        const splBefore = await getAccount(rpc, splAta);
        expect(splBefore.amount).toBe(BigInt(500));

        // Freeze the SPL ATA
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        // Frozen SPL → splBalance=0; no cold → nothing to load
        const result = await loadAta(rpc, ctokenAta, owner, mint, payer, undefined, undefined, true);
        expect(result).toBeNull();

        // SPL account still frozen with original balance
        const splAfter = await getAccount(rpc, splAta);
        expect(splAfter.amount).toBe(BigInt(500));
        expect(splAfter.isFrozen).toBe(true);
    }, 90_000);

    it('createLoadAtaInstructions returns [] when SPL frozen, no cold (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(300), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(300)));
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey, undefined, true,
        );
        expect(batches.length).toBe(0);
    }, 90_000);

    it('frozen SPL excluded, cold unfrozen → only cold is decompressed (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        // Set up: 500 SPL (will freeze) + 400 cold compressed
        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(500), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(500), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)));
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(400), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        // Freeze the SPL ATA
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey, undefined, true,
        );
        // Should build instructions for the 400 cold (SPL is frozen → excluded)
        expect(batches.length).toBeGreaterThan(0);

        for (const batch of batches) {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(batch, payer, blockhash, [owner]);
            await sendAndConfirmTx(rpc, tx);
        }

        // Cold decompressed to hot
        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(400));

        // SPL ATA still frozen with 500
        const splAfter = await getAccount(rpc, splAta);
        expect(splAfter.amount).toBe(BigInt(500));
        expect(splAfter.isFrozen).toBe(true);

        // No cold remaining
        const coldAfter = await getCompressedBalance(rpc, owner.publicKey, mint);
        expect(coldAfter).toBe(BigInt(0));
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

    it('returns null when T22 source is frozen and no cold exists (wrap=true)', async () => {
        const { getOrCreateAssociatedTokenAccount } = await import('@solana/spl-token');
        const owner = await newAccountWithLamports(rpc, 1e9);

        const t22AtaAccount = await getOrCreateAssociatedTokenAccount(rpc, payer as Keypair, t22Mint, owner.publicKey, false, 'confirmed', undefined, TOKEN_2022_PROGRAM_ID);
        const t22Ata = t22AtaAccount.address;

        await mintTo(rpc, payer, t22Mint, owner.publicKey, mintAuthority, bn(500), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(rpc, payer, t22Mint, bn(500), owner, t22Ata, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)));

        await freezeSplAta(rpc, payer, t22Ata, t22Mint, freezeAuthority, TOKEN_2022_PROGRAM_ID);

        const ctokenAta = getAssociatedTokenAddressInterface(t22Mint, owner.publicKey);
        const result = await loadAta(rpc, ctokenAta, owner, t22Mint, payer, undefined, undefined, true);
        expect(result).toBeNull();

        const t22After = await getAccount(rpc, t22Ata, undefined, TOKEN_2022_PROGRAM_ID);
        expect(t22After.isFrozen).toBe(true);
    }, 90_000);

    it('frozen T22 excluded, cold unfrozen → only cold decompressed (wrap=true)', async () => {
        const { getOrCreateAssociatedTokenAccount } = await import('@solana/spl-token');
        const owner = await newAccountWithLamports(rpc, 1e9);

        const t22AtaAccount = await getOrCreateAssociatedTokenAccount(rpc, payer as Keypair, t22Mint, owner.publicKey, false, 'confirmed', undefined, TOKEN_2022_PROGRAM_ID);
        const t22Ata = t22AtaAccount.address;

        // 600 T22 (will freeze) + 300 cold
        await mintTo(rpc, payer, t22Mint, owner.publicKey, mintAuthority, bn(600), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, t22Mint);
        await decompress(rpc, payer, t22Mint, bn(600), owner, t22Ata, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(600)));
        await mintTo(rpc, payer, t22Mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        await freezeSplAta(rpc, payer, t22Ata, t22Mint, freezeAuthority, TOKEN_2022_PROGRAM_ID);

        const ctokenAta = getAssociatedTokenAddressInterface(t22Mint, owner.publicKey);
        const result = await loadAta(rpc, ctokenAta, owner, t22Mint, payer, undefined, undefined, true);
        expect(result).not.toBeNull();

        // Cold 300 loaded, T22 600 still frozen
        const hotBalance = await getCTokenBalance(rpc, ctokenAta);
        expect(hotBalance).toBe(BigInt(300));

        const t22After = await getAccount(rpc, t22Ata, undefined, TOKEN_2022_PROGRAM_ID);
        expect(t22After.amount).toBe(BigInt(600));
        expect(t22After.isFrozen).toBe(true);
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

    it('hot ctoken frozen + SPL unfrozen → only SPL is wrapped (wrap=true)', async () => {
        // Hot is frozen → treated as absent by SDK (coldBalance=0 from hot, just wraps SPL)
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        // Load some cold to create the hot ATA, then freeze it
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);

        // Set up unfrozen SPL balance
        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(500), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(500), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(500)));
        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(500));

        // wrap=true with frozen hot → builds SPL wrap instruction
        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey, undefined, true,
        );
        // SPL balance (500) is unfrozen → should build wrap instruction
        expect(batches.length).toBeGreaterThan(0);

        // But submitting will fail because the destination hot ATA is frozen
        // (Transfer2/wrap into frozen ctoken ATA is rejected on-chain)
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(batches[0], payer, blockhash, [owner]);
        await expect(sendAndConfirmTx(rpc, tx)).rejects.toThrow();

        // SPL balance unchanged, hot still frozen at 300
        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(500));
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(300));
    }, 90_000);

    it('SPL frozen + cold unfrozen → cold decompressed, SPL excluded (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);

        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(400), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(400), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(400)));
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(250), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
        const result = await loadAta(rpc, ctokenAta, owner, mint, payer, undefined, undefined, true);
        expect(result).not.toBeNull();

        // Hot should have 250 (cold only; SPL 400 still frozen)
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(250));
        expect((await getAccount(rpc, splAta)).amount).toBe(BigInt(400));
        expect((await getAccount(rpc, splAta)).isFrozen).toBe(true);
    }, 90_000);

    it('all sources frozen → createLoadAtaInstructions returns [] (wrap=true)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        // Freeze hot ctoken ATA
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(200), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        // Freeze SPL ATA
        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        await decompress(rpc, payer, mint, bn(300), owner, splAta, selectTokenPoolInfosForDecompression(tokenPoolInfos, bn(300)));
        await freezeSplAta(rpc, payer, splAta, mint, freezeAuthority);

        // No cold remaining
        // All sources frozen → nothing to load
        const batches = await createLoadAtaInstructions(
            rpc, ctokenAta, owner.publicKey, mint, payer.publicKey, undefined, true,
        );
        expect(batches.length).toBe(0);

        const loadResult = await loadAta(
            rpc, ctokenAta, owner, mint, payer, undefined, undefined, true,
        );
        expect(loadResult).toBeNull();
    }, 90_000);

    it('all sources frozen → loadAta returns null (wrap=false)', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(400), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        // No cold left; hot is frozen
        const result = await loadAta(rpc, ctokenAta, owner, mint, payer);
        expect(result).toBeNull();
    }, 90_000);

    it('frozen hot does not prevent independent cold→SPL decompress (standard wrap=false, SPL ATA target)', async () => {
        // Scenario: hot ctoken ATA is frozen. User wants to loadAta targeting
        // a plain SPL ATA (wrap=false, ataType='spl'). Cold should still be
        // decompressed directly to the SPL ATA without involving the frozen
        // hot ctoken ATA.
        const owner = await newAccountWithLamports(rpc, 1e9);
        const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);

        // Create and freeze the hot ctoken ATA
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(200), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await loadAta(rpc, ctokenAta, owner, mint, payer);
        await freezeCtokenAta(rpc, payer, ctokenAta, mint, freezeAuthority);

        // Mint 300 more cold compressed
        await mintTo(rpc, payer, mint, owner.publicKey, mintAuthority, bn(300), stateTreeInfo, selectTokenPoolInfo(tokenPoolInfos));

        // Target: SPL ATA (not the frozen ctoken ATA)
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        const splAta = await createAssociatedTokenAccount(rpc, payer, mint, owner.publicKey);

        // Load cold → SPL ATA (wrap=false, direct decompress to SPL)
        const result = await loadAta(rpc, splAta, owner, mint, payer);
        expect(result).not.toBeNull();

        // Cold 300 decompressed to SPL ATA
        const splBalance = await getAccount(rpc, splAta);
        expect(splBalance.amount).toBe(BigInt(300));

        // Hot ctoken ATA still frozen with 200
        expect(await getCTokenBalance(rpc, ctokenAta)).toBe(BigInt(200));
        expect(await getCtokenState(rpc, ctokenAta)).toBe(2);
    }, 90_000);
});
