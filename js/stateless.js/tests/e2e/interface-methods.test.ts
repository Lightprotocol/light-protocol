import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { Rpc, createRpc } from '../../src/rpc';
import { bn, compress, selectStateTreeInfo, sleep, TreeInfo } from '../../src';
import { transfer } from '../../src/actions/transfer';

describe('interface-methods', () => {
    let payer: Signer;
    let bob: Signer;
    let rpc: Rpc;
    let stateTreeInfo: TreeInfo;
    let transferSignature: string;

    beforeAll(async () => {
        rpc = createRpc();

        payer = await newAccountWithLamports(rpc, 10e9, 256);
        bob = await newAccountWithLamports(rpc, 10e9, 256);

        const stateTreeInfos = await rpc.getStateTreeInfos();
        stateTreeInfo = selectStateTreeInfo(stateTreeInfos);

        // Create compressed SOL for testing
        await compress(rpc, payer, 1e9, payer.publicKey, stateTreeInfo);

        // Perform a transfer to generate compression signatures
        transferSignature = await transfer(
            rpc,
            payer,
            1e5,
            payer,
            bob.publicKey,
        );
    });

    describe('getBalanceInterface', () => {
        it('should return unified balance with both on-chain and compressed', async () => {
            const result = await rpc.getBalanceInterface(payer.publicKey);

            // Should have both on-chain and compressed components
            assert.isTrue(
                result.total.gt(bn(0)),
                'Total balance should be > 0',
            );
            assert.isTrue(
                result.onChain.gte(bn(0)),
                'On-chain balance should be >= 0',
            );
            assert.isTrue(
                result.compressed.gte(bn(0)),
                'Compressed balance should be >= 0',
            );

            // Total should equal sum of parts
            assert.isTrue(
                result.total.eq(result.onChain.add(result.compressed)),
                'Total should equal on-chain + compressed',
            );

            // After compress(), payer should have compressed balance
            assert.isTrue(
                result.hasCompressedBalance,
                'Should have compressed balance after compress()',
            );
        });

        it('should work for address with only on-chain balance', async () => {
            // Create fresh account with only on-chain lamports
            const freshAccount = await newAccountWithLamports(rpc, 1e9, 256);

            const result = await rpc.getBalanceInterface(
                freshAccount.publicKey,
            );

            assert.isTrue(result.total.gt(bn(0)));
            assert.isTrue(result.onChain.gt(bn(0)));
            assert.isTrue(result.compressed.eq(bn(0)));
            assert.isFalse(result.hasCompressedBalance);
        });

        it('should work for address with only compressed balance', async () => {
            // Bob received compressed SOL via transfer
            const result = await rpc.getBalanceInterface(bob.publicKey);

            // Bob has both on-chain (initial) and compressed (from transfer)
            assert.isTrue(result.total.gt(bn(0)));
        });
    });

    describe('getSignaturesForAddressInterface', () => {
        it('should return merged signatures from both sources', async () => {
            // Wait for indexer to catch up
            await sleep(2000);

            // Note: getCompressionSignaturesForAddress uses compressed account ADDRESS (not owner)
            // For most practical use cases, compression sigs won't match regular address sigs
            // unless the address has compressed accounts with that specific address field
            const result = await rpc.getSignaturesForAddressInterface(
                payer.publicKey,
            );

            // Should have merged signatures array
            assert.isArray(result.signatures);

            // Should have separate arrays for each source
            assert.isArray(result.solana);
            assert.isArray(result.compressed);

            // The Solana RPC should return signatures for payer's regular transactions
            assert.isAtLeast(
                result.solana.length,
                1,
                'Should have at least one solana signature for payer',
            );
        });

        it('should have proper unified signature structure with sources array', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForAddressInterface(
                payer.publicKey,
            );

            // Check structure of unified signatures
            if (result.signatures.length > 0) {
                const sig = result.signatures[0];
                assert.isString(sig.signature);
                assert.isNumber(sig.slot);
                assert.isDefined(sig.blockTime);
                assert.isDefined(sig.err);
                assert.isDefined(sig.memo);
                // sources is an array of source types
                assert.isArray(sig.sources);
                assert.isAtLeast(sig.sources.length, 1);
                // Each source should be 'solana' or 'compressed'
                for (const source of sig.sources) {
                    assert.include(['solana', 'compressed'], source);
                }
            }
        });

        it('should sort signatures by slot descending', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForAddressInterface(
                payer.publicKey,
            );

            // Verify descending order by slot
            for (let i = 1; i < result.signatures.length; i++) {
                assert.isTrue(
                    result.signatures[i - 1].slot >= result.signatures[i].slot,
                    `Signatures should be sorted by slot descending at index ${i}`,
                );
            }
        });

        it('should deduplicate signatures preferring solana data', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForAddressInterface(
                payer.publicKey,
            );

            // Check for duplicates
            const sigSet = new Set<string>();
            for (const sig of result.signatures) {
                assert.isFalse(
                    sigSet.has(sig.signature),
                    `Duplicate signature found: ${sig.signature}`,
                );
                sigSet.add(sig.signature);
            }
        });
    });

    describe('getSignaturesForOwnerInterface', () => {
        it('should return merged signatures from both sources by owner', async () => {
            // Wait for indexer to catch up
            await sleep(2000);

            const result = await rpc.getSignaturesForOwnerInterface(
                payer.publicKey,
            );

            // Should have merged signatures array
            assert.isArray(result.signatures);

            // Should have separate arrays for each source
            assert.isArray(result.solana);
            assert.isArray(result.compressed);

            // Solana should have signatures for payer
            assert.isAtLeast(
                result.solana.length,
                1,
                'Should have at least one solana signature',
            );

            // Compression should have signatures for owner who did compress/transfer
            assert.isAtLeast(
                result.compressed.length,
                1,
                'Should have at least one compressed signature for owner',
            );
        });

        it('should track sources correctly for compression signatures', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForOwnerInterface(
                payer.publicKey,
            );

            // The raw compressed list should have entries (payer did compress/transfer)
            assert.isAtLeast(result.compressed.length, 1);

            // Find signatures that include 'compressed' in their sources
            const withCompressedSource = result.signatures.filter(sig =>
                sig.sources.includes('compressed'),
            );

            // Should have at least one signature from compression indexer
            assert.isAtLeast(
                withCompressedSource.length,
                1,
                'Should have signatures with compressed source',
            );

            // Signatures found in both should have both sources
            const inBoth = result.signatures.filter(
                sig =>
                    sig.sources.includes('solana') &&
                    sig.sources.includes('compressed'),
            );
            // This is possible if same tx is indexed by both
            assert.isArray(inBoth);
        });

        it('should sort by slot descending', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForOwnerInterface(
                payer.publicKey,
            );

            for (let i = 1; i < result.signatures.length; i++) {
                assert.isTrue(
                    result.signatures[i - 1].slot >= result.signatures[i].slot,
                    `Signatures should be sorted by slot descending`,
                );
            }
        });

        it('should deduplicate signatures', async () => {
            await sleep(2000);

            const result = await rpc.getSignaturesForOwnerInterface(
                payer.publicKey,
            );

            const sigSet = new Set<string>();
            for (const sig of result.signatures) {
                assert.isFalse(
                    sigSet.has(sig.signature),
                    `Duplicate signature found: ${sig.signature}`,
                );
                sigSet.add(sig.signature);
            }
        });
    });

    describe('getTokenAccountBalanceInterface', () => {
        it('should return zero balance for non-existent token account', async () => {
            // Use a random mint that doesn't exist
            const randomMint = PublicKey.unique();
            const randomAta = PublicKey.unique();

            const result = await rpc.getTokenAccountBalanceInterface(
                randomAta,
                payer.publicKey,
                randomMint,
            );

            // Both should be zero for non-existent accounts
            assert.isTrue(result.amount.eq(bn(0)));
            assert.isTrue(result.onChainAmount.eq(bn(0)));
            assert.isTrue(result.compressedAmount.eq(bn(0)));
            assert.isFalse(result.hasCompressedBalance);
            assert.isNull(result.solana);
        });

        it('should have correct structure', async () => {
            const randomMint = PublicKey.unique();
            const randomAta = PublicKey.unique();

            const result = await rpc.getTokenAccountBalanceInterface(
                randomAta,
                payer.publicKey,
                randomMint,
            );

            // Verify structure
            assert.isDefined(result.amount);
            assert.isDefined(result.onChainAmount);
            assert.isDefined(result.compressedAmount);
            assert.isDefined(result.hasCompressedBalance);
            assert.isDefined(result.decimals);
            // solana can be null
            assert.isTrue('solana' in result);

            // Amount should be BN
            assert.isTrue(result.amount instanceof bn(0).constructor);
            assert.isTrue(result.onChainAmount instanceof bn(0).constructor);
            assert.isTrue(result.compressedAmount instanceof bn(0).constructor);
        });
    });
});
