import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import { padOutputStateMerkleTrees } from '../../../src/instruction/pack-compressed-accounts';

describe('padOutputStateMerkleTrees', () => {
    const treeA: any = PublicKey.unique();
    const treeB: any = PublicKey.unique();
    const treeC: any = PublicKey.unique();

    const accA: any = { merkleTree: treeA };
    const accB: any = { merkleTree: treeB };
    const accC: any = { merkleTree: treeC };

    it('should use the 0th state tree of input state if no output state trees are provided', () => {
        const result = padOutputStateMerkleTrees(undefined, 3, [accA, accB]);
        expect(result).toEqual([treeA, treeA, treeA]);
    });

    it('should fill up with the first state tree if provided trees are less than required', () => {
        const result = padOutputStateMerkleTrees([treeA, treeB], 5, []);
        expect(result).toEqual([treeA, treeB, treeA, treeA, treeA]);
    });

    it('should remove extra trees if the number of output state trees is greater than the number of output accounts', () => {
        const result = padOutputStateMerkleTrees([treeA, treeB, treeC], 2, []);
        expect(result).toEqual([treeA, treeB]);
    });

    it('should return the same outputStateMerkleTrees if its length equals the number of output compressed accounts', () => {
        const result = padOutputStateMerkleTrees([treeA, treeB, treeC], 3, []);
        expect(result).toEqual([treeA, treeB, treeC]);
    });
});
