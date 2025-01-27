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

import { Buffer } from 'buffer';
import { struct, u8, u32, array, publicKey, u64 } from '@coral-xyz/borsh';

export const AppendNullifyCreateAddressInputsMetaLayout = struct(
    [
        u8('is_invoked_by_program'),
        u8('bump'),
        u8('num_queues'),
        u8('num_output_queues'),
        u8('start_output_appends'),
        u8('num_address_queues'),
        array(u8(), 32, 'tx_hash'),
    ],
    'appendNullifyCreateAddressInputsMeta',
);

export const AppendLeavesInputLayout = struct(
    [u8('index'), array(u8(), 32, 'leaf')],
    'appendLeavesInput',
);

export const InsertNullifierInputLayout = struct(
    [
        array(u8(), 32, 'account_hash'),
        u32('leaf_index'),
        u8('prove_by_index'),
        u8('tree_index'),
        u8('queue_index'),
    ],
    'insertNullifierInput',
);
export const InsertAddressInputLayout = struct(
    [array(u8(), 32, 'address'), u8('tree_index'), u8('queue_index')],
    'insertAddressInput',
);

export const MerkleTreeSequenceNumberLayout = struct(
    [publicKey('pubkey'), u64('seq')],
    'merkleTreeSequenceNumber',
);

export function deserializeAppendNullifyCreateAddressInputsIndexer(
    buffer: Buffer,
) {
    let offset = 0;
    const meta = AppendNullifyCreateAddressInputsMetaLayout.decode(
        buffer,
        offset,
    );
    offset += AppendNullifyCreateAddressInputsMetaLayout.span;
    console.log('post meta offset', offset);
    const leavesCount = buffer.readUInt8(offset);
    offset += 1;
    const leaves = [];
    for (let i = 0; i < leavesCount; i++) {
        const leaf = AppendLeavesInputLayout.decode(buffer, offset);
        leaves.push(leaf);
        offset += AppendLeavesInputLayout.span;
    }
    console.log('post leaves offset', offset);

    const nullifiersCount = buffer.readUInt8(offset);
    offset += 1;
    const nullifiers = [];
    for (let i = 0; i < nullifiersCount; i++) {
        const nullifier = InsertNullifierInputLayout.decode(buffer, offset);
        nullifiers.push(nullifier);
        offset += InsertNullifierInputLayout.span;
    }
    console.log('post nullifiers offset', offset);
    const addressesCount = buffer.readUInt8(offset);
    offset += 1;
    const addresses = [];
    for (let i = 0; i < addressesCount; i++) {
        const address = InsertAddressInputLayout.decode(buffer, offset);
        addresses.push(address);
        offset += InsertAddressInputLayout.span;
    }
    console.log('post nullifiers offset', offset);
    const sequenceNumbersCount = buffer.readUInt8(offset);
    offset += 1;
    const sequence_numbers = [];
    for (let i = 0; i < sequenceNumbersCount; i++) {
        const seq = MerkleTreeSequenceNumberLayout.decode(buffer, offset);
        sequence_numbers.push(seq);
        offset += MerkleTreeSequenceNumberLayout.span;
    }
    const outputLeafIndicesCount = buffer.readUInt8(offset);
    offset += 1;
    const output_leaf_indices = [];
    for (let i = 0; i < outputLeafIndicesCount; i++) {
        const index = u32().decode(buffer, offset);
        output_leaf_indices.push(index);
        offset += 4;
    }
    return {
        meta,
        leaves,
        nullifiers,
        addresses,
        sequence_numbers,
        output_leaf_indices,
    };
}

describe('deserialize apc cpi', () => {
    const acp_cpi = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
        0,
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
