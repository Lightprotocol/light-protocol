import { Buffer } from 'buffer';
import { PublicKey, AccountMeta } from '@solana/web3.js';
import {
    struct,
    u8,
    u64,
    bool,
    vec,
    option,
    publicKey,
    array,
    u16,
    u32,
    Layout,
    vecU8,
} from '@coral-xyz/borsh';
import {
    bn,
    InstructionDataInvoke,
    InstructionDataInvokeCpi,
    PublicTransactionEvent,
} from '../../state';
import { LightSystemProgram } from '.';
import {
    INVOKE_CPI_DISCRIMINATOR,
    INVOKE_CPI_WITH_READ_ONLY_DISCRIMINATOR,
    INVOKE_DISCRIMINATOR,
} from '../../constants';

export const CompressedAccountLayout = struct(
    [
        publicKey('owner'),
        u64('lamports'),
        option(array(u8(), 32), 'address'),
        option(
            struct([
                array(u8(), 8, 'discriminator'),
                vecU8('data'),
                array(u8(), 32, 'dataHash'),
            ]),
            'data',
        ),
    ],
    'compressedAccount',
);

export const MerkleContextLayout = struct(
    [
        u8('merkleTreePubkeyIndex'),
        u8('queuePubkeyIndex'),
        u32('leafIndex'),
        bool('proveByIndex'),
    ],
    'merkleContext',
);

export const NewAddressParamsLayout = struct(
    [
        array(u8(), 32, 'seed'),
        u8('addressQueueAccountIndex'),
        u8('addressMerkleTreeAccountIndex'),
        u16('addressMerkleTreeRootIndex'),
    ],
    'newAddressParams',
);

export const InstructionDataInvokeLayout: Layout<InstructionDataInvoke> =
    struct([
        option(
            struct([
                array(u8(), 32, 'a'),
                array(u8(), 64, 'b'),
                array(u8(), 32, 'c'),
            ]),
            'proof',
        ),
        vec(
            struct([
                CompressedAccountLayout,
                MerkleContextLayout,
                u16('rootIndex'),
                bool('readOnly'),
            ]),
            'inputCompressedAccountsWithMerkleContext',
        ),
        vec(
            struct([CompressedAccountLayout, u8('merkleTreeIndex')]),
            'outputCompressedAccounts',
        ),
        option(u64(), 'relayFee'),
        vec(NewAddressParamsLayout, 'newAddressParams'),
        option(u64(), 'compressOrDecompressLamports'),
        bool('isCompress'),
    ]);

export function encodeInstructionDataInvoke(
    data: InstructionDataInvoke,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = InstructionDataInvokeLayout.encode(data, buffer);
    const dataBuffer = Buffer.from(new Uint8Array(buffer.slice(0, len)));
    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);
    return Buffer.concat([
        new Uint8Array(INVOKE_DISCRIMINATOR),
        new Uint8Array(lengthBuffer),
        new Uint8Array(dataBuffer),
    ]);
}

export const InstructionDataInvokeCpiLayout: Layout<InstructionDataInvokeCpi> =
    struct([
        option(
            struct([
                array(u8(), 32, 'a'),
                array(u8(), 64, 'b'),
                array(u8(), 32, 'c'),
            ]),
            'proof',
        ),
        vec(NewAddressParamsLayout, 'newAddressParams'),
        vec(
            struct([
                CompressedAccountLayout,
                MerkleContextLayout,
                u16('rootIndex'),
                bool('readOnly'),
            ]),
            'inputCompressedAccountsWithMerkleContext',
        ),
        vec(
            struct([CompressedAccountLayout, u8('merkleTreeIndex')]),
            'outputCompressedAccounts',
        ),
        option(u64(), 'relayFee'),
        option(u64(), 'compressOrDecompressLamports'),
        bool('isCompress'),
        option(
            struct([
                bool('set_context'),
                bool('first_set_context'),
                u8('cpi_context_account_index'),
            ]),
            'compressedCpiContext',
        ),
    ]);

export const CompressedProofLayout = struct(
    [array(u8(), 32, 'a'), array(u8(), 64, 'b'), array(u8(), 32, 'c')],
    'compressedProof',
);

export const CompressedCpiContextLayout = struct(
    [
        bool('set_context'),
        bool('first_set_context'),
        u8('cpi_context_account_index'),
    ],
    'compressedCpiContext',
);

export const NewAddressParamsAssignedPackedLayout = struct(
    [
        array(u8(), 32, 'seed'),
        u8('address_queue_account_index'),
        u8('address_merkle_tree_account_index'),
        u16('address_merkle_tree_root_index'),
        bool('assigned_to_account'),
        u8('assigned_account_index'),
    ],
    'newAddressParamsAssignedPacked',
);

export const PackedMerkleContextLayout = struct(
    [
        u8('merkle_tree_pubkey_index'),
        u8('queue_pubkey_index'),
        u32('leaf_index'),
        bool('prove_by_index'),
    ],
    'packedMerkleContext',
);

export const InAccountLayout = struct(
    [
        array(u8(), 8, 'discriminator'),
        array(u8(), 32, 'data_hash'),
        PackedMerkleContextLayout,
        u16('root_index'),
        u64('lamports'),
        option(array(u8(), 32), 'address'),
    ],
    'inAccount',
);

export const PackedReadOnlyAddressLayout = struct(
    [
        array(u8(), 32, 'address'),
        u16('address_merkle_tree_root_index'),
        u8('address_merkle_tree_account_index'),
    ],
    'packedReadOnlyAddress',
);

export const PackedReadOnlyCompressedAccountLayout = struct(
    [
        array(u8(), 32, 'account_hash'),
        PackedMerkleContextLayout,
        u16('root_index'),
    ],
    'packedReadOnlyCompressedAccount',
);

export const InstructionDataInvokeCpiWithReadOnlyLayout = struct([
    u8('mode'),
    u8('bump'),
    publicKey('invoking_program_id'),
    u64('compress_or_decompress_lamports'),
    bool('is_compress'),
    bool('with_cpi_context'),
    bool('with_transaction_hash'),
    CompressedCpiContextLayout,
    option(CompressedProofLayout, 'proof'),
    vec(NewAddressParamsAssignedPackedLayout, 'new_address_params'),
    vec(InAccountLayout, 'input_compressed_accounts'),
    vec(
        struct([CompressedAccountLayout, u8('merkleTreeIndex')]),
        'output_compressed_accounts',
    ),
    vec(PackedReadOnlyAddressLayout, 'read_only_addresses'),
    vec(PackedReadOnlyCompressedAccountLayout, 'read_only_accounts'),
]);

export function decodeInstructionDataInvokeCpiWithReadOnly(buffer: Buffer) {
    return InstructionDataInvokeCpiWithReadOnlyLayout.decode(
        buffer.slice(INVOKE_CPI_WITH_READ_ONLY_DISCRIMINATOR.length),
    );
}

export function decodeInstructionDataInvoke(
    buffer: Buffer,
): InstructionDataInvoke {
    return InstructionDataInvokeLayout.decode(
        buffer.slice(INVOKE_DISCRIMINATOR.length + 4),
    );
}

export function decodeInstructionDataInvokeCpi(
    buffer: Buffer,
): InstructionDataInvokeCpi {
    return InstructionDataInvokeCpiLayout.decode(
        buffer.slice(INVOKE_CPI_DISCRIMINATOR.length + 4),
    );
}

export type invokeAccountsLayoutParams = {
    /**
     * Fee payer.
     */
    feePayer: PublicKey;
    /**
     * Authority.
     */
    authority: PublicKey;
    /**
     * The registered program pda
     */
    registeredProgramPda: PublicKey;
    /**
     * Noop program.
     */
    noopProgram: PublicKey;
    /**
     * Account compression authority.
     */
    accountCompressionAuthority: PublicKey;
    /**
     * Account compression program.
     */
    accountCompressionProgram: PublicKey;
    /**
     * Solana pool pda. Some() if compression or decompression is done.
     */
    solPoolPda: PublicKey | null;
    /**
     * Decompression recipient.
     */
    decompressionRecipient: PublicKey | null;
    /**
     * Solana system program.
     */
    systemProgram: PublicKey;
};

export const invokeAccountsLayout = (
    accounts: invokeAccountsLayoutParams,
): AccountMeta[] => {
    const defaultPubkey = LightSystemProgram.programId;
    const {
        feePayer,
        authority,
        registeredProgramPda,
        noopProgram,
        accountCompressionAuthority,
        accountCompressionProgram,
        solPoolPda,
        decompressionRecipient,
        systemProgram,
    } = accounts;

    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: registeredProgramPda, isSigner: false, isWritable: false },
        { pubkey: noopProgram, isSigner: false, isWritable: false },
        {
            pubkey: accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: solPoolPda ?? defaultPubkey,
            isSigner: false,
            isWritable: solPoolPda !== null,
        },
        {
            pubkey: decompressionRecipient ?? defaultPubkey,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
    ];
};

export const PublicTransactionEventLayout: Layout<PublicTransactionEvent> =
    struct([
        vec(array(u8(), 32), 'inputCompressedAccountHashes'),
        vec(array(u8(), 32), 'outputCompressedAccountHashes'),
        vec(
            struct([
                struct(
                    [
                        publicKey('owner'),
                        u64('lamports'),
                        option(array(u8(), 32), 'address'),
                        option(
                            struct([
                                array(u8(), 8, 'discriminator'),
                                vecU8('data'),
                                array(u8(), 32, 'dataHash'),
                            ]),
                            'data',
                        ),
                    ],
                    'compressedAccount',
                ),
                u8('merkleTreeIndex'),
            ]),
            'outputCompressedAccounts',
        ),
        vec(u32(), 'outputLeafIndices'),
        vec(
            struct([
                publicKey('tree_pubkey'),
                publicKey('queue_pubkey'),
                u64('tree_type'),
                u64('seq'),
            ]),
            'sequenceNumbers',
        ),
        option(u64(), 'relayFee'),
        bool('isCompress'),
        option(u64(), 'compressOrDecompressLamports'),
        vec(publicKey(), 'pubkeyArray'),
        option(vecU8(), 'message'),
    ]);

export function encodePublicTransactionEvent(
    data: PublicTransactionEvent,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = PublicTransactionEventLayout.encode(data, buffer);

    return buffer.slice(0, len);
}

export function decodePublicTransactionEvent(
    buffer: Buffer,
): PublicTransactionEvent {
    return PublicTransactionEventLayout.decode(buffer);
}

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
    [
        publicKey('tree_pubkey'),
        publicKey('queue_pubkey'),
        u64('tree_type'),
        u64('seq'),
    ],
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
    const leavesCount = buffer.readUInt8(offset);
    offset += 1;
    const leaves = [];
    for (let i = 0; i < leavesCount; i++) {
        const leaf = AppendLeavesInputLayout.decode(buffer, offset);
        leaves.push(leaf);
        offset += AppendLeavesInputLayout.span;
    }
    const nullifiersCount = buffer.readUInt8(offset);
    offset += 1;
    const nullifiers = [];
    for (let i = 0; i < nullifiersCount; i++) {
        const nullifier = InsertNullifierInputLayout.decode(buffer, offset);
        nullifiers.push(nullifier);
        offset += InsertNullifierInputLayout.span;
    }
    const addressesCount = buffer.readUInt8(offset);
    offset += 1;
    const addresses = [];
    for (let i = 0; i < addressesCount; i++) {
        const address = InsertAddressInputLayout.decode(buffer, offset);
        addresses.push(address);
        offset += InsertAddressInputLayout.span;
    }
    const outputSequenceNumbersCount = buffer.readUInt8(offset);
    offset += 1;
    const output_sequence_numbers = [];
    for (let i = 0; i < outputSequenceNumbersCount; i++) {
        const seq = MerkleTreeSequenceNumberLayout.decode(buffer, offset);
        output_sequence_numbers.push(seq);
        offset += MerkleTreeSequenceNumberLayout.span;
    }
    const inputSequenceNumbersCount = buffer.readUInt8(offset);
    offset += 1;
    const inputSequence_numbers = [];
    for (let i = 0; i < inputSequenceNumbersCount; i++) {
        const seq = MerkleTreeSequenceNumberLayout.decode(buffer, offset);
        inputSequence_numbers.push(seq);
        offset += MerkleTreeSequenceNumberLayout.span;
    }
    const addressSequenceNumbersCount = buffer.readUInt8(offset);
    offset += 1;
    const addressSequence_numbers = [];
    for (let i = 0; i < addressSequenceNumbersCount; i++) {
        const seq = MerkleTreeSequenceNumberLayout.decode(buffer, offset);
        addressSequence_numbers.push(seq);
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
        sequence_numbers: output_sequence_numbers,
        output_leaf_indices,
    };
}

export function convertToPublicTransactionEvent(
    decoded: any,
    remainingAccounts: PublicKey[],
    invokeData: InstructionDataInvoke,
): PublicTransactionEvent {
    const convertByteArray = (arr: Uint8Array | Buffer): number[] =>
        Array.from(arr instanceof Buffer ? new Uint8Array(arr) : arr);

    const result = {
        inputCompressedAccountHashes: decoded.nullifiers.map((n: any) =>
            convertByteArray(n.account_hash),
        ),
        outputCompressedAccountHashes: decoded.leaves.map((l: any) =>
            convertByteArray(l.leaf),
        ),
        outputCompressedAccounts: decoded.leaves.map(
            (leaf: any, index: number) => ({
                compressedAccount: {
                    owner: new PublicKey(
                        invokeData?.outputCompressedAccounts[index]
                            ?.compressedAccount.owner || PublicKey.default,
                    ),
                    lamports: bn(
                        invokeData?.outputCompressedAccounts[index]
                            ?.compressedAccount.lamports || 0,
                    ),
                    address:
                        invokeData?.outputCompressedAccounts[index]
                            .compressedAccount.address,
                    data: invokeData?.outputCompressedAccounts[index]
                        ?.compressedAccount.data
                        ? {
                              discriminator: convertByteArray(
                                  Buffer.from(
                                      invokeData.outputCompressedAccounts[index]
                                          .compressedAccount.data
                                          ?.discriminator,
                                  ),
                              ),
                              data:
                                  convertByteArray(
                                      Buffer.from(
                                          new Uint8Array(
                                              invokeData.outputCompressedAccounts[
                                                  index
                                              ].compressedAccount.data.data,
                                          ),
                                      ),
                                  ) ?? [],
                              dataHash: convertByteArray(
                                  Buffer.from(
                                      invokeData.outputCompressedAccounts[index]
                                          .compressedAccount.data?.dataHash,
                                  ),
                              ),
                          }
                        : null,
                },
                merkleTreeIndex: leaf.index,
            }),
        ),
        outputLeafIndices: decoded.output_leaf_indices,
        sequenceNumbers: decoded.sequence_numbers.map((sn: any) => {
            return {
                tree_pubkey: sn.tree_pubkey,
                queue_pubkey: sn.queue_pubkey,
                tree_type: sn.tree_type,
                seq: sn.seq,
            };
        }),
        pubkeyArray: remainingAccounts
            .slice(2)
            .filter(pk => !pk.equals(PublicKey.default)),
        isCompress: invokeData?.isCompress || false,
        relayFee: invokeData?.relayFee ? bn(invokeData.relayFee) : null,
        compressOrDecompressLamports: invokeData?.compressOrDecompressLamports
            ? bn(invokeData.compressOrDecompressLamports)
            : null,
        message: null,
    };

    return result;
}
