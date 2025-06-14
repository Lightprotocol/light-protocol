import { describe, expect, it } from 'vitest';
import { TreeType, TreeInfo } from '../../../src/state';
import { selectStateTreeInfo } from '../../../src/utils';
import { PublicKey } from '@solana/web3.js';
import {
    cpiContext2Pubkey,
    cpiContextPubkey,
    merkleTree2Pubkey,
    merkletreePubkey,
    nullifierQueue2Pubkey,
    nullifierQueuePubkey,
} from '../../../src';

describe('selectStateTreeInfo', () => {
    const info1: TreeInfo = {
        tree: new PublicKey(merkletreePubkey),
        queue: new PublicKey(nullifierQueuePubkey),
        cpiContext: new PublicKey(cpiContextPubkey),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const info2: TreeInfo = {
        tree: new PublicKey(merkleTree2Pubkey),
        queue: new PublicKey(nullifierQueue2Pubkey),
        cpiContext: new PublicKey(cpiContext2Pubkey),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const infoV2: TreeInfo = {
        tree: new PublicKey(merkleTree2Pubkey),
        queue: new PublicKey(nullifierQueue2Pubkey),
        cpiContext: new PublicKey(cpiContext2Pubkey),
        treeType: TreeType.StateV2,
        nextTreeInfo: null,
    };
    const info3: TreeInfo = {
        tree: PublicKey.unique(),
        queue: PublicKey.unique(),
        cpiContext: PublicKey.unique(),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const info4: TreeInfo = {
        tree: PublicKey.unique(),
        queue: PublicKey.unique(),
        cpiContext: PublicKey.unique(),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const info5: TreeInfo = {
        tree: PublicKey.unique(),
        queue: PublicKey.unique(),
        cpiContext: PublicKey.unique(),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const info6: TreeInfo = {
        tree: PublicKey.unique(),
        queue: PublicKey.unique(),
        cpiContext: PublicKey.unique(),
        treeType: TreeType.StateV1,
        nextTreeInfo: null,
    };
    const infoInactive: TreeInfo = {
        tree: PublicKey.unique(),
        queue: PublicKey.unique(),
        cpiContext: PublicKey.unique(),
        treeType: TreeType.StateV1,
        nextTreeInfo: info1,
    };

    const info1V2: TreeInfo = {
        tree: new PublicKey(merkletreePubkey),
        queue: new PublicKey(nullifierQueuePubkey),
        cpiContext: new PublicKey(cpiContextPubkey),
        treeType: TreeType.StateV2,
        nextTreeInfo: null,
    };

    it('returns a filtered tree info', () => {
        const infos = [info1, info2, infoV2];
        for (let i = 0; i < 10_000; i++) {
            const result = selectStateTreeInfo(infos, TreeType.StateV1);
            expect(result.treeType).toBe(TreeType.StateV1);
        }
    });

    it('should default to MAX_HOTSPOTS (5) if there are more than 5 infos', () => {
        const infos = [info1, info2, info3, info4, info5, info6];
        for (let i = 0; i < 10_000; i++) {
            const result = selectStateTreeInfo(infos, TreeType.StateV1);
            const expectedRange = [info1, info2, info3, info4, info5];

            expect(expectedRange.length).toBe(5);
            expect(expectedRange.includes(result)).toBe(true);
        }
    });

    it('should return all infos if useMaxConcurrency is true', () => {
        const infos = [info1, info2, info3, info4, info5, info6];
        for (let i = 0; i < 10_000; i++) {
            const result = selectStateTreeInfo(infos, TreeType.StateV1, true);
            const expectedRange = [info1, info2, info3, info4, info5, info6];

            expect(expectedRange.includes(result)).toBe(true);
        }
    });

    it('should never return inactive infos if useMaxConcurrency is true', () => {
        const infos = [info1, info2, info3, info4, info5, info6, infoInactive];
        for (let i = 0; i < 100_000; i++) {
            const result = selectStateTreeInfo(infos, TreeType.StateV1, true);
            const expectedRange = [info1, info2, info3, info4, info5, info6];

            expect(expectedRange.includes(result)).toBe(true);
            expect(result !== infoInactive).toBe(true);
        }
    });

    it('throws if queue is missing', () => {
        const infos = [
            { ...info1, queue: null },
            { ...info1V2, queue: null },
        ];
        expect(() => selectStateTreeInfo(infos as any)).toThrow(
            'Queue must not be null for state tree',
        );
    });
});
