/**
 * E2E tests for load functions with a real indexer.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
    getTestRpc,
    fundAccount,
    createTestMint,
    mintCompressedTokens,
    toKitAddress,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    PhotonIndexer,
    loadTokenAccountsForTransfer,
    loadAllTokenAccounts,
    loadTokenAccount,
    needsValidityProof,
    getOutputTreeInfo,
    getTreeInfo,
} from '../../src/index.js';

const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('load functions e2e', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: any;
    let mintAuthority: Signer;
    let indexer: PhotonIndexer;

    beforeAll(async () => {
        rpc = getTestRpc();
        payer = await fundAccount(rpc);

        const created = await createTestMint(rpc, payer, DECIMALS);
        mint = created.mint;
        mintAuthority = created.mintAuthority;

        await mintCompressedTokens(
            rpc, payer, mint, payer.publicKey, mintAuthority, MINT_AMOUNT,
        );

        indexer = new PhotonIndexer(COMPRESSION_RPC);
    });

    it('loadTokenAccountsForTransfer returns accounts + proof', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        const loaded = await loadTokenAccountsForTransfer(
            indexer,
            ownerAddr,
            5_000n,
            { mint: mintAddr },
        );

        expect(loaded.inputs.length).toBeGreaterThan(0);
        expect(loaded.totalAmount).toBeGreaterThanOrEqual(5_000n);
        expect(loaded.proof).toBeDefined();

        // Verify input structure
        const input = loaded.inputs[0];
        expect(input.tokenAccount).toBeDefined();
        expect(input.merkleContext.tree).toBeDefined();
        expect(input.merkleContext.queue).toBeDefined();
        expect(typeof input.merkleContext.leafIndex).toBe('number');
    });

    it('loadAllTokenAccounts returns all accounts', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        const accounts = await loadAllTokenAccounts(indexer, ownerAddr, {
            mint: mintAddr,
        });

        expect(accounts.length).toBeGreaterThan(0);
        expect(accounts[0].token.mint).toBe(mintAddr);
    });

    it('loadTokenAccount returns single account', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        const account = await loadTokenAccount(indexer, ownerAddr, mintAddr);

        expect(account).not.toBeNull();
        expect(account!.token.mint).toBe(mintAddr);
        expect(account!.token.owner).toBe(ownerAddr);
    });

    it('loadTokenAccount returns null for unknown mint', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const { address } = await import('@solana/addresses');
        const fakeMint = address('FakeMint111111111111111111111111111111111111');

        const account = await loadTokenAccount(indexer, ownerAddr, fakeMint);
        expect(account).toBeNull();
    });

    it('needsValidityProof / getTreeInfo / getOutputTreeInfo with real data', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        const accounts = await loadAllTokenAccounts(indexer, ownerAddr, {
            mint: mintAddr,
        });
        const account = accounts[0];

        // needsValidityProof
        const needsProof = needsValidityProof(account.account);
        expect(typeof needsProof).toBe('boolean');

        // getTreeInfo
        const treeInfo = getTreeInfo(account.account);
        expect(treeInfo.tree).toBeDefined();
        expect(treeInfo.queue).toBeDefined();

        // getOutputTreeInfo - should return current or next tree
        const outputTree = getOutputTreeInfo(treeInfo);
        expect(outputTree.tree).toBeDefined();
        expect(outputTree.queue).toBeDefined();
    });
});
