/**
 * Unit tests for compute-unit estimation and batch splitting utilities.
 *
 * H1: calculateLoadBatchComputeUnits – exported, no unit test existed
 * H2: calculateTransferCU             – internal, exported for testing
 * H3: sliceLast                       – exported utility, no dedicated test
 */
import { describe, it, expect } from 'vitest';
import { Keypair, TransactionInstruction } from '@solana/web3.js';
import { TreeType } from '@lightprotocol/stateless.js';
import {
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from '../../src/v3/actions/load-ata';
import {
    calculateTransferCU,
    sliceLast,
} from '../../src/v3/actions/transfer-interface';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mockParsedAccount(proveByIndex: boolean): any {
    return {
        parsed: {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: { toString: () => '100' },
            delegate: null,
            state: 1,
            tlv: null,
        },
        compressedAccount: {
            hash: new Uint8Array(32),
            treeInfo: {
                tree: Keypair.generate().publicKey,
                queue: Keypair.generate().publicKey,
                treeType: TreeType.StateV2,
            },
            leafIndex: 0,
            proveByIndex,
            owner: Keypair.generate().publicKey,
            lamports: { toString: () => '0' },
            address: null,
            data: null,
            readOnly: false,
        },
    };
}

function emptyBatch(overrides: Partial<InternalLoadBatch> = {}): InternalLoadBatch {
    return {
        instructions: [],
        compressedAccounts: [],
        wrapCount: 0,
        hasAtaCreation: false,
        ...overrides,
    };
}

function fakeIx(): TransactionInstruction {
    return new TransactionInstruction({
        programId: Keypair.generate().publicKey,
        keys: [],
        data: Buffer.alloc(0),
    });
}

// ---------------------------------------------------------------------------
// H1: calculateLoadBatchComputeUnits
// ---------------------------------------------------------------------------

describe('calculateLoadBatchComputeUnits', () => {
    it('returns min 50_000 for empty batch', () => {
        const cu = calculateLoadBatchComputeUnits(emptyBatch());
        expect(cu).toBe(50_000);
    });

    it('adds 30_000 for ATA creation', () => {
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ hasAtaCreation: true }),
        );
        // 30_000 * 1.3 = 39_000 → clamped to 50_000 min
        expect(cu).toBe(50_000);
    });

    it('adds 50_000 per wrap', () => {
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ wrapCount: 2 }),
        );
        // 100_000 * 1.3 = 130_000
        expect(cu).toBe(130_000);
    });

    it('base decompress cost: 50_000 + full proof (100_000) + per-account 30_000 for 1 full-proof account', () => {
        const acc = mockParsedAccount(false);
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: [acc] }),
        );
        // (50_000 + 100_000 + 30_000) * 1.3 = 234_000
        expect(cu).toBe(234_000);
    });

    it('proveByIndex accounts: 10_000 per account, no full-proof overhead', () => {
        const acc = mockParsedAccount(true);
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: [acc] }),
        );
        // (50_000 + 10_000) * 1.3 = 78_000
        expect(cu).toBe(78_000);
    });

    it('mixed: one proveByIndex + one full-proof triggers full-proof overhead', () => {
        const fullProof = mockParsedAccount(false);
        const byIndex = mockParsedAccount(true);
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: [fullProof, byIndex] }),
        );
        // (50_000 + 100_000 + 30_000 + 10_000) * 1.3 = 247_000
        expect(cu).toBe(247_000);
    });

    it('8 full-proof accounts: (50_000 + 100_000 + 8*30_000) * 1.3 = 507_000', () => {
        const accounts = Array.from({ length: 8 }, () => mockParsedAccount(false));
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: accounts }),
        );
        expect(cu).toBe(507_000);
    });

    it('8 proveByIndex accounts: (50_000 + 8*10_000) * 1.3 = 169_000', () => {
        const accounts = Array.from({ length: 8 }, () => mockParsedAccount(true));
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: accounts }),
        );
        expect(cu).toBe(169_000);
    });

    it('ATA creation + 1 wrap + 1 full-proof account', () => {
        const acc = mockParsedAccount(false);
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({
                hasAtaCreation: true,
                wrapCount: 1,
                compressedAccounts: [acc],
            }),
        );
        // (30_000 + 50_000 + 50_000 + 100_000 + 30_000) * 1.3 = 338_000
        expect(cu).toBe(338_000);
    });

    it('caps at 1_400_000 for extreme inputs', () => {
        const accounts = Array.from({ length: 100 }, () => mockParsedAccount(false));
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: accounts }),
        );
        expect(cu).toBe(1_400_000);
    });

    it('30% buffer is applied (result is ceiling of n*1.3)', () => {
        // 1 proveByIndex account: (50_000 + 10_000) = 60_000, * 1.3 = 78_000 (exact)
        const acc = mockParsedAccount(true);
        const cu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: [acc] }),
        );
        expect(cu).toBe(Math.ceil(60_000 * 1.3));
    });
});

// ---------------------------------------------------------------------------
// H2: calculateTransferCU
// ---------------------------------------------------------------------------

describe('calculateTransferCU', () => {
    it('hot sender (null batch): 10_000 base * 1.3 = 13_000 → clamped to 50_000', () => {
        const cu = calculateTransferCU(null);
        expect(cu).toBe(50_000);
    });

    it('empty load batch: 10_000 base * 1.3 = 13_000 → clamped to 50_000', () => {
        const cu = calculateTransferCU(emptyBatch());
        expect(cu).toBe(50_000);
    });

    it('ATA creation in batch: (10_000 + 30_000) * 1.3 = 52_000', () => {
        const cu = calculateTransferCU(emptyBatch({ hasAtaCreation: true }));
        expect(cu).toBe(52_000);
    });

    it('1 wrap in batch: (10_000 + 50_000) * 1.3 = 78_000', () => {
        const cu = calculateTransferCU(emptyBatch({ wrapCount: 1 }));
        expect(cu).toBe(78_000);
    });

    it('1 full-proof compressed account: (10_000 + 50_000 + 100_000 + 30_000) * 1.3 = 247_000', () => {
        const acc = mockParsedAccount(false);
        const cu = calculateTransferCU(emptyBatch({ compressedAccounts: [acc] }));
        expect(cu).toBe(247_000);
    });

    it('1 proveByIndex account: (10_000 + 50_000 + 10_000) * 1.3 = 91_000', () => {
        const acc = mockParsedAccount(true);
        const cu = calculateTransferCU(emptyBatch({ compressedAccounts: [acc] }));
        expect(cu).toBe(91_000);
    });

    it('8 full-proof accounts: (10_000 + 50_000 + 100_000 + 8*30_000) * 1.3 = 520_000', () => {
        const accounts = Array.from({ length: 8 }, () => mockParsedAccount(false));
        const cu = calculateTransferCU(emptyBatch({ compressedAccounts: accounts }));
        expect(cu).toBe(520_000);
    });

    it('ATA + 1 wrap + 8 full-proof: combines all costs', () => {
        const accounts = Array.from({ length: 8 }, () => mockParsedAccount(false));
        const cu = calculateTransferCU(
            emptyBatch({
                hasAtaCreation: true,
                wrapCount: 1,
                compressedAccounts: accounts,
            }),
        );
        // (10_000 + 30_000 + 50_000 + 50_000 + 100_000 + 8*30_000) * 1.3
        // = (10_000+30_000+50_000+50_000+100_000+240_000) * 1.3
        // = 480_000 * 1.3 = 624_000
        expect(cu).toBe(624_000);
    });

    it('caps at 1_400_000', () => {
        const accounts = Array.from({ length: 100 }, () => mockParsedAccount(false));
        const cu = calculateTransferCU(emptyBatch({ compressedAccounts: accounts }));
        expect(cu).toBe(1_400_000);
    });

    it('transfer CU exceeds load CU (transfer adds 10_000 base)', () => {
        const acc = mockParsedAccount(false);
        const loadCu = calculateLoadBatchComputeUnits(
            emptyBatch({ compressedAccounts: [acc] }),
        );
        const transferCu = calculateTransferCU(
            emptyBatch({ compressedAccounts: [acc] }),
        );
        expect(transferCu).toBeGreaterThan(loadCu);
    });
});

// ---------------------------------------------------------------------------
// H3: sliceLast
// ---------------------------------------------------------------------------

describe('sliceLast', () => {
    it('throws for empty array', () => {
        expect(() => sliceLast([])).toThrow('sliceLast: array must not be empty');
    });

    it('single element: rest=[], last=element', () => {
        const ix = fakeIx();
        const result = sliceLast([[ix]]);
        expect(result.rest).toHaveLength(0);
        expect(result.last).toStrictEqual([ix]);
    });

    it('two elements: rest=[first], last=second', () => {
        const ix1 = [fakeIx()];
        const ix2 = [fakeIx(), fakeIx()];
        const result = sliceLast([ix1, ix2]);
        expect(result.rest).toHaveLength(1);
        expect(result.rest[0]).toBe(ix1);
        expect(result.last).toBe(ix2);
    });

    it('five elements: rest has 4, last is 5th', () => {
        const items = [1, 2, 3, 4, 5];
        const result = sliceLast(items);
        expect(result.rest).toEqual([1, 2, 3, 4]);
        expect(result.last).toBe(5);
    });

    it('does not mutate the input array', () => {
        const original = [1, 2, 3];
        const copy = [...original];
        sliceLast(original);
        expect(original).toEqual(copy);
    });

    it('rest is a new array (not the original)', () => {
        const items = [1, 2, 3];
        const { rest } = sliceLast(items);
        expect(rest).not.toBe(items);
    });
});
