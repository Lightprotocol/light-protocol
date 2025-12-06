import {
    struct,
    option,
    vec,
    bool,
    u64,
    u8,
    u16,
    u32,
    array,
    vecU8,
} from '@coral-xyz/borsh';
import { Buffer } from 'buffer';
import { ValidityProof } from '@lightprotocol/stateless.js';
import { DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR } from '../../constants';

const ValidityProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

const PackedStateTreeInfoLayout = struct([
    u16('rootIndex'),
    bool('proveByIndex'),
    u8('merkleTreePubkeyIndex'),
    u8('queuePubkeyIndex'),
    u32('leafIndex'),
]);

const CompressedAccountMetaLayout = struct([
    PackedStateTreeInfoLayout.replicate('treeInfo'),
    option(array(u8(), 32), 'address'),
    option(u64(), 'lamports'),
    u8('outputStateTreeIndex'),
]);

export interface PackedStateTreeInfo {
    rootIndex: number;
    proveByIndex: boolean;
    merkleTreePubkeyIndex: number;
    queuePubkeyIndex: number;
    leafIndex: number;
}

export interface CompressedAccountMeta {
    treeInfo: PackedStateTreeInfo;
    address: number[] | null;
    lamports: bigint | null;
    outputStateTreeIndex: number;
}

export interface CompressedAccountData<T = any> {
    meta: CompressedAccountMeta;
    data: T;
    seeds: Uint8Array[];
}

export interface DecompressAccountsIdempotentInstructionData<T = any> {
    proof: ValidityProof;
    compressedAccounts: CompressedAccountData<T>[];
    systemAccountsOffset: number;
}

export function createCompressedAccountDataLayout<T>(dataLayout: any): any {
    return struct([
        CompressedAccountMetaLayout.replicate('meta'),
        dataLayout.replicate('data'),
        vec(vecU8(), 'seeds'),
    ]);
}

export function createDecompressAccountsIdempotentLayout<T>(
    dataLayout: any,
): any {
    return struct([
        ValidityProofLayout.replicate('proof'),
        vec(
            createCompressedAccountDataLayout(dataLayout),
            'compressedAccounts',
        ),
        u8('systemAccountsOffset'),
    ]);
}

/**
 * Serialize decompress idempotent instruction data
 * @param data       The decompress idempotent instruction data
 * @param dataLayout The data layout
 * @returns The serialized decompress idempotent instruction data
 */
export function serializeDecompressIdempotentInstructionData<T = any>(
    data: DecompressAccountsIdempotentInstructionData<T>,
    dataLayout: any,
): Buffer {
    const layout = createDecompressAccountsIdempotentLayout(dataLayout);
    const buffer = Buffer.alloc(1000);

    const len = layout.encode(data, buffer);

    return Buffer.concat([
        DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        buffer.subarray(0, len),
    ]);
}

/**
 * Deserialize decompress idempotent instruction data
 * @param buffer The serialized decompress idempotent instruction data
 * @param dataLayout The data layout
 * @returns The decompress idempotent instruction data
 */
export function deserializeDecompressIdempotentInstructionData<T = any>(
    buffer: Buffer,
    dataLayout: any,
): DecompressAccountsIdempotentInstructionData<T> {
    const layout = createDecompressAccountsIdempotentLayout(dataLayout);
    return layout.decode(
        buffer.subarray(DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR.length),
    ) as DecompressAccountsIdempotentInstructionData<T>;
}
