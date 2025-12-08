import { describe, it, expect } from 'vitest';
import { mergeSignatures } from '../../../src/rpc';
import { SignatureSource } from '../../../src/rpc-interface';
import type { ConfirmedSignatureInfo } from '@solana/web3.js';
import type { SignatureWithMetadata } from '../../../src/rpc-interface';

describe('mergeSignatures', () => {
    describe('empty inputs', () => {
        it('should return empty array when both inputs are empty', () => {
            const result = mergeSignatures([], []);
            expect(result).toEqual([]);
        });
    });

    describe('solana-only signatures', () => {
        it('should return solana signatures with solana source', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig1',
                    slot: 100,
                    blockTime: 1700000000,
                    err: null,
                    memo: 'test memo',
                    confirmationStatus: 'finalized',
                },
            ];

            const result = mergeSignatures(solanaSignatures, []);

            expect(result).toHaveLength(1);
            expect(result[0]).toEqual({
                signature: 'sig1',
                slot: 100,
                blockTime: 1700000000,
                err: null,
                memo: 'test memo',
                confirmationStatus: 'finalized',
                sources: [SignatureSource.Solana],
            });
        });

        it('should handle null blockTime from solana', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig1',
                    slot: 100,
                    blockTime: null,
                    err: null,
                    memo: null,
                    confirmationStatus: 'confirmed',
                },
            ];

            const result = mergeSignatures(solanaSignatures, []);

            expect(result[0].blockTime).toBeNull();
        });

        it('should handle undefined memo from solana', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig1',
                    slot: 100,
                    blockTime: 1700000000,
                    err: null,
                    memo: undefined as unknown as string | null,
                    confirmationStatus: 'confirmed',
                },
            ];

            const result = mergeSignatures(solanaSignatures, []);

            expect(result[0].memo).toBeNull();
        });

        it('should preserve error info from solana', () => {
            const errorObj = { InstructionError: [0, 'Custom'] };
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig1',
                    slot: 100,
                    blockTime: 1700000000,
                    err: errorObj,
                    memo: null,
                    confirmationStatus: 'finalized',
                },
            ];

            const result = mergeSignatures(solanaSignatures, []);

            expect(result[0].err).toEqual(errorObj);
        });
    });

    describe('compressed-only signatures', () => {
        it('should return compressed signatures with compressed source', () => {
            const compressedSignatures: SignatureWithMetadata[] = [
                {
                    signature: 'csig1',
                    slot: 200,
                    blockTime: 1700001000,
                },
            ];

            const result = mergeSignatures([], compressedSignatures);

            expect(result).toHaveLength(1);
            expect(result[0]).toEqual({
                signature: 'csig1',
                slot: 200,
                blockTime: 1700001000,
                err: null,
                memo: null,
                confirmationStatus: undefined,
                sources: [SignatureSource.Compressed],
            });
        });

        it('should handle multiple compressed signatures', () => {
            const compressedSignatures: SignatureWithMetadata[] = [
                { signature: 'csig1', slot: 200, blockTime: 1700001000 },
                { signature: 'csig2', slot: 150, blockTime: 1700000500 },
                { signature: 'csig3', slot: 250, blockTime: 1700001500 },
            ];

            const result = mergeSignatures([], compressedSignatures);

            expect(result).toHaveLength(3);
            result.forEach(sig => {
                expect(sig.sources).toEqual([SignatureSource.Compressed]);
            });
        });
    });

    describe('deduplication', () => {
        it('should dedupe signature found in both sources and mark both sources', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'shared_sig',
                    slot: 100,
                    blockTime: 1700000000,
                    err: null,
                    memo: 'solana memo',
                    confirmationStatus: 'finalized',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                {
                    signature: 'shared_sig',
                    slot: 100,
                    blockTime: 1700000000,
                },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result).toHaveLength(1);
            expect(result[0].signature).toBe('shared_sig');
            expect(result[0].sources).toEqual([
                SignatureSource.Solana,
                SignatureSource.Compressed,
            ]);
        });

        it('should use solana data (richer) when signature exists in both', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'shared_sig',
                    slot: 100,
                    blockTime: 1700000000,
                    err: { SomeError: 'test' },
                    memo: 'important memo',
                    confirmationStatus: 'finalized',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                {
                    signature: 'shared_sig',
                    slot: 100,
                    blockTime: 1700000000,
                },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result[0].memo).toBe('important memo');
            expect(result[0].err).toEqual({ SomeError: 'test' });
            expect(result[0].confirmationStatus).toBe('finalized');
        });

        it('should not create duplicates when same signature appears in both', () => {
            const sig = 'duplicate_test_sig';
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: sig,
                    slot: 500,
                    blockTime: 1700005000,
                    err: null,
                    memo: null,
                    confirmationStatus: 'confirmed',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                { signature: sig, slot: 500, blockTime: 1700005000 },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            const matching = result.filter(r => r.signature === sig);
            expect(matching).toHaveLength(1);
        });
    });

    describe('mixed sources', () => {
        it('should correctly merge signatures from both sources', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'solana_only',
                    slot: 300,
                    blockTime: 1700003000,
                    err: null,
                    memo: null,
                    confirmationStatus: 'finalized',
                },
                {
                    signature: 'shared',
                    slot: 200,
                    blockTime: 1700002000,
                    err: null,
                    memo: 'shared memo',
                    confirmationStatus: 'confirmed',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                {
                    signature: 'compressed_only',
                    slot: 400,
                    blockTime: 1700004000,
                },
                { signature: 'shared', slot: 200, blockTime: 1700002000 },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result).toHaveLength(3);

            const solanaOnly = result.find(r => r.signature === 'solana_only');
            expect(solanaOnly?.sources).toEqual([SignatureSource.Solana]);

            const compressedOnly = result.find(
                r => r.signature === 'compressed_only',
            );
            expect(compressedOnly?.sources).toEqual([
                SignatureSource.Compressed,
            ]);

            const shared = result.find(r => r.signature === 'shared');
            expect(shared?.sources).toEqual([
                SignatureSource.Solana,
                SignatureSource.Compressed,
            ]);
            expect(shared?.memo).toBe('shared memo');
        });
    });

    describe('sorting', () => {
        it('should sort by slot descending (most recent first)', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig_slot_100',
                    slot: 100,
                    blockTime: 1700001000,
                    err: null,
                    memo: null,
                    confirmationStatus: 'finalized',
                },
                {
                    signature: 'sig_slot_300',
                    slot: 300,
                    blockTime: 1700003000,
                    err: null,
                    memo: null,
                    confirmationStatus: 'finalized',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                { signature: 'sig_slot_200', slot: 200, blockTime: 1700002000 },
                { signature: 'sig_slot_400', slot: 400, blockTime: 1700004000 },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result.map(r => r.slot)).toEqual([400, 300, 200, 100]);
            expect(result.map(r => r.signature)).toEqual([
                'sig_slot_400',
                'sig_slot_300',
                'sig_slot_200',
                'sig_slot_100',
            ]);
        });

        it('should maintain stable order for same slot', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = [
                {
                    signature: 'sig_a',
                    slot: 100,
                    blockTime: 1700001000,
                    err: null,
                    memo: null,
                    confirmationStatus: 'finalized',
                },
            ];
            const compressedSignatures: SignatureWithMetadata[] = [
                { signature: 'sig_b', slot: 100, blockTime: 1700001000 },
            ];

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result).toHaveLength(2);
            expect(result.every(r => r.slot === 100)).toBe(true);
        });
    });

    describe('edge cases', () => {
        it('should handle large number of signatures', () => {
            const solanaSignatures: ConfirmedSignatureInfo[] = Array.from(
                { length: 100 },
                (_, i) => ({
                    signature: `solana_sig_${i}`,
                    slot: i * 10,
                    blockTime: 1700000000 + i,
                    err: null,
                    memo: null,
                    confirmationStatus: 'finalized' as const,
                }),
            );
            const compressedSignatures: SignatureWithMetadata[] = Array.from(
                { length: 100 },
                (_, i) => ({
                    signature: `compressed_sig_${i}`,
                    slot: i * 10 + 5,
                    blockTime: 1700000000 + i,
                }),
            );

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result).toHaveLength(200);
            for (let i = 1; i < result.length; i++) {
                expect(result[i - 1].slot).toBeGreaterThanOrEqual(
                    result[i].slot,
                );
            }
        });

        it('should handle many duplicate signatures', () => {
            const sharedSigs = Array.from(
                { length: 50 },
                (_, i) => `shared_${i}`,
            );

            const solanaSignatures: ConfirmedSignatureInfo[] = sharedSigs.map(
                (sig, i) => ({
                    signature: sig,
                    slot: i * 10,
                    blockTime: 1700000000 + i,
                    err: null,
                    memo: `memo_${i}`,
                    confirmationStatus: 'finalized' as const,
                }),
            );
            const compressedSignatures: SignatureWithMetadata[] =
                sharedSigs.map((sig, i) => ({
                    signature: sig,
                    slot: i * 10,
                    blockTime: 1700000000 + i,
                }));

            const result = mergeSignatures(
                solanaSignatures,
                compressedSignatures,
            );

            expect(result).toHaveLength(50);
            result.forEach(r => {
                expect(r.sources).toEqual([
                    SignatureSource.Solana,
                    SignatureSource.Compressed,
                ]);
            });
        });
    });
});
