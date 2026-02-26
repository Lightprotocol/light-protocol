/**
 * E2E tests for PhotonIndexer against a real endpoint.
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
    createLightIndexer,
    isLightIndexerAvailable,
} from '../../src/index.js';

const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const DECIMALS = 2;
const MINT_AMOUNT = 10_000n;

describe('PhotonIndexer e2e', () => {
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

        // Mint tokens so there's something to query
        await mintCompressedTokens(
            rpc, payer, mint, payer.publicKey, mintAuthority, MINT_AMOUNT,
        );

        indexer = new PhotonIndexer(COMPRESSION_RPC);
    });

    it('isLightIndexerAvailable returns true for running endpoint', async () => {
        const available = await isLightIndexerAvailable(COMPRESSION_RPC);
        expect(available).toBe(true);
    });

    it('isLightIndexerAvailable returns false for invalid endpoint', async () => {
        const available = await isLightIndexerAvailable(
            'http://127.0.0.1:9999',
        );
        expect(available).toBe(false);
    });

    it('getCompressedTokenAccountsByOwner returns token accounts', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        const response = await indexer.getCompressedTokenAccountsByOwner(
            ownerAddr,
            { mint: mintAddr },
        );

        expect(response.value.items.length).toBeGreaterThan(0);
        const account = response.value.items[0];
        expect(account.token.mint).toBe(mintAddr);
        expect(account.token.owner).toBe(ownerAddr);
        expect(account.token.amount).toBe(MINT_AMOUNT);
        expect(account.account.hash).toBeInstanceOf(Uint8Array);
    });

    it('getValidityProof returns valid proof', async () => {
        const ownerAddr = toKitAddress(payer.publicKey);
        const mintAddr = toKitAddress(mint);

        // First get an account to prove
        const accountsResponse =
            await indexer.getCompressedTokenAccountsByOwner(ownerAddr, {
                mint: mintAddr,
            });
        const account = accountsResponse.value.items[0];

        const proofResponse = await indexer.getValidityProof([
            account.account.hash,
        ]);

        expect(proofResponse.value).toBeDefined();
        expect(proofResponse.value.accounts.length).toBeGreaterThan(0);
    });

    it('createLightIndexer factory works', () => {
        const client = createLightIndexer(COMPRESSION_RPC);
        expect(client).toBeDefined();
        expect(typeof client.getCompressedTokenAccountsByOwner).toBe(
            'function',
        );
        expect(typeof client.getValidityProof).toBe('function');
    });
});
