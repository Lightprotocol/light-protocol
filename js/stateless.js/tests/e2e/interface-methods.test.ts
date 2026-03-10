import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import {
    createMint,
    getMint,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
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
    let splMint: PublicKey;
    let token2022Mint: PublicKey;

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

        splMint = await createMint(
            rpc,
            payer,
            payer.publicKey,
            null,
            6,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        token2022Mint = await createMint(
            rpc,
            payer,
            payer.publicKey,
            null,
            9,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
    });

    describe('getBalanceInterface', () => {
        it('should return unified balance', async () => {
            const result = await rpc.getBalanceInterface(payer.publicKey);

            assert.isTrue(
                result.total.gt(bn(0)),
                'Total balance should be > 0',
            );

            // After compress(), payer should have cold balance
            assert.isTrue(
                result.hasColdBalance,
                'Should have cold balance after compress()',
            );
        });

        it('should work for address with only hot balance', async () => {
            const freshAccount = await newAccountWithLamports(rpc, 1e9, 256);

            const result = await rpc.getBalanceInterface(
                freshAccount.publicKey,
            );

            assert.isTrue(result.total.gt(bn(0)));
            assert.isFalse(result.hasColdBalance);
        });

        it('should work for address with cold balance', async () => {
            const result = await rpc.getBalanceInterface(bob.publicKey);

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

            // Should be zero for non-existent accounts
            assert.isTrue(result.amount.eq(bn(0)));
            assert.isFalse(result.hasColdBalance);
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
            assert.isDefined(result.hasColdBalance);
            assert.isDefined(result.decimals);
            assert.isTrue('solana' in result);

            // Amount should be BN
            assert.isTrue(result.amount instanceof bn(0).constructor);
        });
    });

    describe('raw account interface endpoints', () => {
        it('getAccountInterface should return SPL mint raw account data', async () => {
            const result = await rpc.getAccountInterface(splMint);
            const mint = await getMint(rpc, splMint, 'confirmed', TOKEN_PROGRAM_ID);

            expect(result.context.slot).toBeGreaterThan(0);
            expect(result.value).not.toBeNull();
            expect(result.value!.key.toBase58()).toBe(splMint.toBase58());
            expect(result.value!.account.owner.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.value!.account.data.length).toBeGreaterThan(0);
            expect(result.value!.account.data.length).toBe(82);
            expect(mint.decimals).toBe(6);
            expect(mint.mintAuthority?.toBase58()).toBe(payer.publicKey.toBase58());
        });

        it('getAccountInterface should return Token-2022 mint raw account data', async () => {
            const result = await rpc.getAccountInterface(token2022Mint);
            const mint = await getMint(
                rpc,
                token2022Mint,
                'confirmed',
                TOKEN_2022_PROGRAM_ID,
            );

            expect(result.context.slot).toBeGreaterThan(0);
            expect(result.value).not.toBeNull();
            expect(result.value!.key.toBase58()).toBe(token2022Mint.toBase58());
            expect(result.value!.account.owner.toBase58()).toBe(
                TOKEN_2022_PROGRAM_ID.toBase58(),
            );
            expect(result.value!.account.data.length).toBeGreaterThan(0);
            expect(result.value!.account.data.length).toBe(82);
            expect(mint.decimals).toBe(9);
            expect(mint.mintAuthority?.toBase58()).toBe(payer.publicKey.toBase58());
        });

        it('getAccountInterface should return raw account envelope for existing address', async () => {
            const result = await rpc.getAccountInterface(payer.publicKey);

            expect(result.context.slot).toBeGreaterThan(0);
            expect(result.value).not.toBeNull();
            expect(result.value!.key.toBase58()).toBe(payer.publicKey.toBase58());
            expect(result.value!.account.owner.toBase58()).toBe(
                PublicKey.default.toBase58(),
            );
            expect(Buffer.isBuffer(result.value!.account.data)).toBe(true);
        });

        it('getAccountInterface should return null for missing address', async () => {
            const missing = PublicKey.unique();
            const result = await rpc.getAccountInterface(missing);

            expect(result.context.slot).toBeGreaterThan(0);
            expect(result.value).toBeNull();
        });

        it('getMultipleAccountInterfaces should preserve order and nullability', async () => {
            const missing = PublicKey.unique();
            const result = await rpc.getMultipleAccountInterfaces([
                splMint,
                token2022Mint,
                missing,
            ]);

            expect(result.context.slot).toBeGreaterThan(0);
            expect(result.value.length).toBe(3);
            expect(result.value[0]).not.toBeNull();
            expect(result.value[0]!.key.toBase58()).toBe(splMint.toBase58());
            expect(result.value[1]).not.toBeNull();
            expect(result.value[1]!.key.toBase58()).toBe(token2022Mint.toBase58());
            expect(result.value[2]).toBeNull();
        });
    });
});
