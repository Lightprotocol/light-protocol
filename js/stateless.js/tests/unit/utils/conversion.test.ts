import { describe, it, expect } from 'vitest';
import { toArray } from '../../../src/utils/conversion';
import { calculateComputeUnitPrice } from '../../../src/utils';
import { deserializeAppendNullifyCreateAddressInputsIndexer } from '../../../src/programs';

describe('toArray', () => {
    it('should return same array if array is passed', () => {
        const arr = [1, 2, 3];
        expect(toArray(arr)).toBe(arr);
    });

    it('should wrap non-array in array', () => {
        const value = 42;
        expect(toArray(value)).toEqual([42]);
    });

    describe('calculateComputeUnitPrice', () => {
        it('calculates correct price', () => {
            expect(calculateComputeUnitPrice(1000, 200000)).toBe(5000); // 1000 lamports / 200k CU = 5000 microlamports/CU
            expect(calculateComputeUnitPrice(100, 50000)).toBe(2000); // 100 lamports / 50k CU = 2000 microlamports/CU
            expect(calculateComputeUnitPrice(1, 1000000)).toBe(1); // 1 lamport / 1M CU = 1 microlamport/CU
        });
    });
});

describe('deserialize apc cpi', () => {
    const acp_cpi = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        0, 0, 0,
    ];

    it('deserialize acp cpi', () => {
        const buffer = Buffer.from(acp_cpi);
        const result =
            deserializeAppendNullifyCreateAddressInputsIndexer(buffer);

        expect(result.meta.is_invoked_by_program).toEqual(1);

        expect(result.addresses.length).toBeGreaterThan(0);
        console.log('address ', result.addresses[0]);
        expect(result.addresses[0]).toEqual({
            address: new Array(32).fill(1),
            tree_index: 1,
            queue_index: 1,
        });

        expect(result.leaves.length).toBeGreaterThan(0);
        expect(result.leaves[0]).toEqual({
            index: 1,
            leaf: new Array(32).fill(1),
        });

        expect(result.nullifiers.length).toBeGreaterThan(0);
        expect(result.nullifiers[0]).toEqual({
            account_hash: new Array(32).fill(1),
            leaf_index: 1,
            prove_by_index: 1,
            tree_index: 1,
            queue_index: 1,
        });
    });
});
