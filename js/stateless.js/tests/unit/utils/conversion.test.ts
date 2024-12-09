import { describe, it, expect } from 'vitest';
import { toArray } from '../../../src/utils/conversion';
import { calculateComputeUnitPrice } from '../../../src/utils';

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
