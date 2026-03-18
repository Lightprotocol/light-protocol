/**
 * E2E tests for MintAction instruction (compressed mint management).
 *
 * Uses V3/stateless.js for setup (mints, mint interface, proofs).
 * Uses token-kit's createMintActionInstruction to build instructions.
 * Verifies results on-chain.
 *
 * Requires a running local validator + indexer + prover.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import type { Address } from '@solana/addresses';
import { AccountRole } from '@solana/instructions';

import {
    getTestRpc,
    fundAccount,
    createTestMint,
    createTestMintWithMetadata,
    sendKitInstructions,
    toKitAddress,
    ensureValidatorRunning,
    type Signer,
    type Rpc,
} from './helpers/setup.js';

import {
    getMintInterface,
    updateMintAuthority,
    updateMetadataField,
} from '@lightprotocol/compressed-token';

import {
    getOutputQueue,
} from '@lightprotocol/stateless.js';

import {
    createMintActionInstruction,
    DISCRIMINATOR,
    type MintActionInstructionData,
} from '../../src/index.js';

const DECIMALS = 2;

describe('MintAction e2e', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        await ensureValidatorRunning();
        rpc = getTestRpc();
        payer = await fundAccount(rpc);
    });

    it('update mint authority on decompressed mint', async () => {
        // Setup: create decompressed CToken mint
        const { mint, mintAuthority, mintAddress } = await createTestMint(
            rpc, payer, DECIMALS,
        );

        // Get mint interface from V3 for merkle context
        const mintInterface = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        expect(mintInterface.merkleContext).toBeDefined();
        expect(mintInterface.mintContext?.cmintDecompressed).toBe(true);

        const merkleContext = mintInterface.merkleContext!;
        const outputQueue = getOutputQueue(merkleContext);

        // Build MintAction instruction via token-kit
        const newAuthority = await fundAccount(rpc);
        const newAuthorityBytes = (newAuthority as any).publicKey.toBytes();

        const data: MintActionInstructionData = {
            leafIndex: merkleContext.leafIndex,
            proveByIndex: true,
            rootIndex: 0, // No proof needed for decompressed mints
            maxTopUp: 0,
            createMint: null,
            actions: [{
                type: 'UpdateMintAuthority',
                newAuthority: new Uint8Array(newAuthorityBytes),
            }],
            proof: null, // No proof for decompressed mints
            cpiContext: null,
            mint: null, // Program reads from CMint account
        };

        const ix = createMintActionInstruction({
            authority: toKitAddress((mintAuthority as any).publicKey),
            feePayer: toKitAddress((payer as any).publicKey),
            outOutputQueue: toKitAddress(outputQueue),
            merkleTree: toKitAddress(merkleContext.treeInfo.tree),
            cmint: mintAddress,
            data,
            packedAccounts: [
                // in_output_queue (required when createMint is null)
                {
                    address: toKitAddress(merkleContext.treeInfo.queue),
                    role: AccountRole.WRITABLE,
                },
            ],
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);

        // Send on-chain
        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        // Verify: re-read mint interface, check authority changed
        const updatedMint = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        const newAuth = (newAuthority as any).publicKey;
        expect(updatedMint.mint.mintAuthority?.equals(newAuth)).toBe(true);
    });

    it('update metadata field on decompressed mint', async () => {
        const { mint, mintAuthority } = await createTestMintWithMetadata(
            rpc, payer, DECIMALS,
        );

        // First verify mint has metadata
        const mintInterface = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        expect(mintInterface.tokenMetadata).toBeDefined();
        const originalName = mintInterface.tokenMetadata!.name;

        // Use V3's updateMetadataField to update the name
        await updateMetadataField(
            rpc,
            payer as any,
            mint,
            mintAuthority as any,
            'name',
            'Updated Name',
        );

        // Verify: re-read mint, check metadata field changed
        const updatedMint = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        expect(updatedMint.tokenMetadata!.name).toBe('Updated Name');
        expect(updatedMint.tokenMetadata!.name).not.toBe(originalName);
    });

    it('update mint authority via V3 action (reference test)', async () => {
        // This test verifies the V3 action works end-to-end,
        // establishing the baseline for token-kit instruction tests.
        const { mint, mintAuthority } = await createTestMint(
            rpc, payer, DECIMALS,
        );
        const newAuthority = await fundAccount(rpc);

        await updateMintAuthority(
            rpc,
            payer as any,
            mint,
            mintAuthority as any,
            (newAuthority as any).publicKey,
        );

        const updatedMint = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        const newAuth = (newAuthority as any).publicKey;
        expect(updatedMint.mint.mintAuthority?.equals(newAuth)).toBe(true);
    });

    it('revoke mint authority (set to null)', async () => {
        const { mint, mintAuthority, mintAddress } = await createTestMint(
            rpc, payer, DECIMALS,
        );

        const mintInterface = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        const merkleContext = mintInterface.merkleContext!;
        const outputQueue = getOutputQueue(merkleContext);

        // Build instruction to revoke (set authority to null)
        const data: MintActionInstructionData = {
            leafIndex: merkleContext.leafIndex,
            proveByIndex: true,
            rootIndex: 0,
            maxTopUp: 0,
            createMint: null,
            actions: [{
                type: 'UpdateMintAuthority',
                newAuthority: null,
            }],
            proof: null,
            cpiContext: null,
            mint: null,
        };

        const ix = createMintActionInstruction({
            authority: toKitAddress((mintAuthority as any).publicKey),
            feePayer: toKitAddress((payer as any).publicKey),
            outOutputQueue: toKitAddress(outputQueue),
            merkleTree: toKitAddress(merkleContext.treeInfo.tree),
            cmint: mintAddress,
            data,
            packedAccounts: [{
                address: toKitAddress(merkleContext.treeInfo.queue),
                role: AccountRole.WRITABLE,
            }],
        });

        await sendKitInstructions(rpc, [ix], payer, [mintAuthority]);

        // Verify: mint authority is now null
        const updatedMint = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        expect(updatedMint.mint.mintAuthority).toBeNull();
    });

    it('update metadata symbol field', async () => {
        const { mint, mintAuthority } = await createTestMintWithMetadata(
            rpc, payer, DECIMALS,
        );

        // Use V3 action for symbol update
        await updateMetadataField(
            rpc,
            payer as any,
            mint,
            mintAuthority as any,
            'symbol',
            'NEWSYM',
        );

        const updatedMint = await getMintInterface(
            rpc, mint, undefined, undefined,
        );
        expect(updatedMint.tokenMetadata!.symbol).toBe('NEWSYM');
    });
});
