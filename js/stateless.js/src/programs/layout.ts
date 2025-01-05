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
import { InstructionDataInvoke, PublicTransactionEvent } from '../state';
import { LightSystemProgram } from './system';
import { INVOKE_DISCRIMINATOR } from '../constants';
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
        u8('nullifierQueuePubkeyIndex'),
        u32('leafIndex'),
        option(struct([u8('queueId'), u16('index')]), 'queueIndex'),
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
    const dataBuffer = Buffer.from(buffer.slice(0, len));

    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);

    return Buffer.concat([INVOKE_DISCRIMINATOR, lengthBuffer, dataBuffer]);
}

export function decodeInstructionDataInvoke(
    buffer: Buffer,
): InstructionDataInvoke {
    return InstructionDataInvokeLayout.decode(buffer);
}

export type invokeAccountsLayoutParams = {
    feePayer: PublicKey;
    authority: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    solPoolPda: PublicKey | null;
    decompressionRecipient: PublicKey | null;
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
        vec(struct([publicKey('pubkey'), u64('seq')]), 'sequenceNumbers'),
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
