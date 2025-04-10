import {
    struct,
    option,
    vec,
    bool,
    u64,
    u8,
    publicKey,
    array,
    u32,
    u16,
    vecU8,
} from '@coral-xyz/borsh';
import { AccountMeta, PublicKey } from '@solana/web3.js';
import { CompressedTokenProgram } from './program';
import {
    BatchCompressInstructionData,
    CompressedTokenInstructionDataApprove,
    CompressedTokenInstructionDataRevoke,
    CompressedTokenInstructionDataTransfer,
    CompressSplTokenAccountInstructionData,
    MintToInstructionData,
} from './types';
import {
    APPROVE_DISCRIMINATOR,
    BATCH_COMPRESS_DISCRIMINATOR,
    COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR,
    MINT_TO_DISCRIMINATOR,
    REVOKE_DISCRIMINATOR,
    TRANSFER_DISCRIMINATOR,
} from './constants';
import { Buffer } from 'buffer';
import { ValidityProof } from '@lightprotocol/stateless.js';

const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

const PackedTokenTransferOutputDataLayout = struct([
    publicKey('owner'),
    u64('amount'),
    option(u64(), 'lamports'),
    u8('merkleTreeIndex'),
    option(vecU8(), 'tlv'),
]);

const InputTokenDataWithContextLayout = struct([
    u64('amount'),
    option(u8(), 'delegateIndex'),
    struct(
        [
            u8('merkleTreePubkeyIndex'),
            u8('queuePubkeyIndex'),
            u32('leafIndex'),
            bool('proveByIndex'),
        ],
        'merkleContext',
    ),
    u16('rootIndex'),
    option(u64(), 'lamports'),
    option(vecU8(), 'tlv'),
]);

export const DelegatedTransferLayout = struct([
    publicKey('owner'),
    option(u8(), 'delegateChangeAccountIndex'),
]);

export const CpiContextLayout = struct([
    bool('setContext'),
    bool('firstSetContext'),
    u8('cpiContextAccountIndex'),
]);

export const CompressedTokenInstructionDataTransferLayout = struct([
    option(CompressedProofLayout, 'proof'),
    publicKey('mint'),
    option(DelegatedTransferLayout, 'delegatedTransfer'),
    vec(InputTokenDataWithContextLayout, 'inputTokenDataWithContext'),
    vec(PackedTokenTransferOutputDataLayout, 'outputCompressedAccounts'),
    bool('isCompress'),
    option(u64(), 'compressOrDecompressAmount'),
    option(CpiContextLayout, 'cpiContext'),
    option(u8(), 'lamportsChangeAccountMerkleTreeIndex'),
]);

export const mintToLayout = struct([
    vec(publicKey(), 'recipients'),
    vec(u64(), 'amounts'),
    option(u64(), 'lamports'),
]);

export const batchCompressLayout = struct([
    vec(publicKey(), 'pubkeys'),
    option(vec(u64(), 'amounts'), 'amounts'),
    option(u64(), 'lamports'),
    option(u64(), 'amount'),
    u8('index'),
    u8('bump'),
]);

export const compressSplTokenAccountInstructionDataLayout = struct([
    publicKey('owner'),
    option(u64(), 'remainingAmount'),
    option(CpiContextLayout, 'cpiContext'),
]);

export function encodeMintToInstructionData(
    data: MintToInstructionData,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = mintToLayout.encode(
        {
            recipients: data.recipients,
            amounts: data.amounts,
            lamports: data.lamports,
        },
        buffer,
    );

    return Buffer.concat([
        new Uint8Array(MINT_TO_DISCRIMINATOR),
        new Uint8Array(buffer.subarray(0, len)),
    ]);
}

export function decodeMintToInstructionData(
    buffer: Buffer,
): MintToInstructionData {
    return mintToLayout.decode(
        buffer.subarray(MINT_TO_DISCRIMINATOR.length),
    ) as MintToInstructionData;
}

export function encodeBatchCompressInstructionData(
    data: BatchCompressInstructionData,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = batchCompressLayout.encode(data, buffer);

    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);

    const dataBuffer = buffer.subarray(0, len);
    return Buffer.concat([
        new Uint8Array(BATCH_COMPRESS_DISCRIMINATOR),
        new Uint8Array(lengthBuffer),
        new Uint8Array(dataBuffer),
    ]);
}

export function decodeBatchCompressInstructionData(
    buffer: Buffer,
): BatchCompressInstructionData {
    return batchCompressLayout.decode(
        buffer.subarray(BATCH_COMPRESS_DISCRIMINATOR.length + 4),
    ) as BatchCompressInstructionData;
}

export function encodeCompressSplTokenAccountInstructionData(
    data: CompressSplTokenAccountInstructionData,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = compressSplTokenAccountInstructionDataLayout.encode(
        {
            owner: data.owner,
            remainingAmount: data.remainingAmount,
            cpiContext: data.cpiContext,
        },
        buffer,
    );

    return Buffer.concat([
        new Uint8Array(COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR),
        new Uint8Array(buffer.subarray(0, len)),
    ]);
}

export function decodeCompressSplTokenAccountInstructionData(
    buffer: Buffer,
): CompressSplTokenAccountInstructionData {
    const data = compressSplTokenAccountInstructionDataLayout.decode(
        buffer.subarray(COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR.length),
    ) as CompressSplTokenAccountInstructionData;
    return {
        owner: data.owner,
        remainingAmount: data.remainingAmount,
        cpiContext: data.cpiContext,
    };
}
export function encodeTransferInstructionData(
    data: CompressedTokenInstructionDataTransfer,
): Buffer {
    const buffer = Buffer.alloc(1000);

    const len = CompressedTokenInstructionDataTransferLayout.encode(
        data,
        buffer,
    );

    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);

    const dataBuffer = buffer.subarray(0, len);

    return Buffer.concat([
        new Uint8Array(TRANSFER_DISCRIMINATOR),
        new Uint8Array(lengthBuffer),
        new Uint8Array(dataBuffer),
    ]);
}

export function decodeTransferInstructionData(
    buffer: Buffer,
): CompressedTokenInstructionDataTransfer {
    return CompressedTokenInstructionDataTransferLayout.decode(
        buffer.slice(TRANSFER_DISCRIMINATOR.length + 4),
    ) as CompressedTokenInstructionDataTransfer;
}

interface BaseAccountsLayoutParams {
    feePayer: PublicKey;
    authority: PublicKey;
    cpiAuthorityPda: PublicKey;
    lightSystemProgram: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    selfProgram: PublicKey;
    systemProgram: PublicKey;
}
export type createTokenPoolAccountsLayoutParams = {
    feePayer: PublicKey;
    tokenPoolPda: PublicKey;
    systemProgram: PublicKey;
    mint: PublicKey;
    tokenProgram: PublicKey;
    cpiAuthorityPda: PublicKey;
};

export type addTokenPoolAccountsLayoutParams =
    createTokenPoolAccountsLayoutParams & {
        existingTokenPoolPda: PublicKey;
    };

export type mintToAccountsLayoutParams = BaseAccountsLayoutParams & {
    mint: PublicKey;
    tokenPoolPda: PublicKey;
    tokenProgram: PublicKey;
    merkleTree: PublicKey;
    solPoolPda: PublicKey | null;
};
export type transferAccountsLayoutParams = BaseAccountsLayoutParams & {
    tokenPoolPda?: PublicKey;
    compressOrDecompressTokenAccount?: PublicKey;
    tokenProgram?: PublicKey;
};
export type approveAccountsLayoutParams = BaseAccountsLayoutParams;
export type revokeAccountsLayoutParams = approveAccountsLayoutParams;
export type freezeAccountsLayoutParams = BaseAccountsLayoutParams & {
    mint: PublicKey;
};
export type thawAccountsLayoutParams = freezeAccountsLayoutParams;

export const createTokenPoolAccountsLayout = (
    accounts: createTokenPoolAccountsLayoutParams,
): AccountMeta[] => {
    const {
        feePayer,
        tokenPoolPda,
        systemProgram,
        mint,
        tokenProgram,
        cpiAuthorityPda,
    } = accounts;
    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: tokenPoolPda, isSigner: false, isWritable: true },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: tokenProgram, isSigner: false, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
    ];
};

export const addTokenPoolAccountsLayout = (
    accounts: addTokenPoolAccountsLayoutParams,
): AccountMeta[] => {
    const {
        feePayer,
        tokenPoolPda,
        systemProgram,
        mint,
        tokenProgram,
        cpiAuthorityPda,
        existingTokenPoolPda,
    } = accounts;
    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: tokenPoolPda, isSigner: false, isWritable: true },
        { pubkey: existingTokenPoolPda, isSigner: false, isWritable: false },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: tokenProgram, isSigner: false, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
    ];
};

export const mintToAccountsLayout = (
    accounts: mintToAccountsLayoutParams,
): AccountMeta[] => {
    const defaultPubkey = CompressedTokenProgram.programId;
    const {
        feePayer,
        authority,
        cpiAuthorityPda,
        mint,
        tokenPoolPda,
        tokenProgram,
        lightSystemProgram,
        registeredProgramPda,
        noopProgram,
        accountCompressionAuthority,
        accountCompressionProgram,
        merkleTree,
        selfProgram,
        systemProgram,
        solPoolPda,
    } = accounts;

    const accountsList: AccountMeta[] = [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: tokenPoolPda, isSigner: false, isWritable: true },
        { pubkey: tokenProgram, isSigner: false, isWritable: false },
        { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
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
        { pubkey: merkleTree, isSigner: false, isWritable: true },
        { pubkey: selfProgram, isSigner: false, isWritable: false },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
        {
            pubkey: solPoolPda ?? defaultPubkey,
            isSigner: false,
            isWritable: true,
        },
    ];

    return accountsList;
};

export const transferAccountsLayout = (
    accounts: transferAccountsLayoutParams,
): AccountMeta[] => {
    const defaultPubkey = CompressedTokenProgram.programId;
    const {
        feePayer,
        authority,
        cpiAuthorityPda,
        lightSystemProgram,
        registeredProgramPda,
        noopProgram,
        accountCompressionAuthority,
        accountCompressionProgram,
        selfProgram,
        tokenPoolPda,
        compressOrDecompressTokenAccount,
        tokenProgram,
        systemProgram,
    } = accounts;

    const accountsList: AccountMeta[] = [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
        { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
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
        { pubkey: selfProgram, isSigner: false, isWritable: false },
        {
            pubkey: tokenPoolPda ?? defaultPubkey,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: compressOrDecompressTokenAccount ?? defaultPubkey,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: tokenProgram ?? defaultPubkey,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
    ];

    return accountsList;
};

export const approveAccountsLayout = (
    accounts: approveAccountsLayoutParams,
): AccountMeta[] => {
    const {
        feePayer,
        authority,
        cpiAuthorityPda,
        lightSystemProgram,
        registeredProgramPda,
        noopProgram,
        accountCompressionAuthority,
        accountCompressionProgram,
        selfProgram,
        systemProgram,
    } = accounts;

    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
        { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
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
        { pubkey: selfProgram, isSigner: false, isWritable: false },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
    ];
};

export const revokeAccountsLayout = approveAccountsLayout;

export const freezeAccountsLayout = (
    accounts: freezeAccountsLayoutParams,
): AccountMeta[] => {
    const {
        feePayer,
        authority,
        cpiAuthorityPda,
        lightSystemProgram,
        registeredProgramPda,
        noopProgram,
        accountCompressionAuthority,
        accountCompressionProgram,
        selfProgram,
        systemProgram,
        mint,
    } = accounts;

    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
        { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
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
        { pubkey: selfProgram, isSigner: false, isWritable: false },
        { pubkey: systemProgram, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
    ];
};

export const thawAccountsLayout = freezeAccountsLayout;

export const CompressedTokenInstructionDataApproveLayout = struct([
    struct(
        [array(u8(), 32, 'a'), array(u8(), 64, 'b'), array(u8(), 32, 'c')],
        'proof',
    ),
    publicKey('mint'),
    vec(InputTokenDataWithContextLayout, 'inputTokenDataWithContext'),
    option(CpiContextLayout, 'cpiContext'),
    publicKey('delegate'),
    u64('delegatedAmount'),
    u8('delegateMerkleTreeIndex'),
    u8('changeAccountMerkleTreeIndex'),
    option(u64(), 'delegateLamports'),
]);

export const CompressedTokenInstructionDataRevokeLayout = struct([
    struct(
        [array(u8(), 32, 'a'), array(u8(), 64, 'b'), array(u8(), 32, 'c')],
        'proof',
    ),
    publicKey('mint'),
    vec(InputTokenDataWithContextLayout, 'inputTokenDataWithContext'),
    option(CpiContextLayout, 'cpiContext'),
    u8('outputAccountMerkleTreeIndex'),
]);

// Approve and revoke instuctions do not support optional proof yet.
const emptyProof: ValidityProof = {
    a: new Array(32).fill(0),
    b: new Array(64).fill(0),
    c: new Array(32).fill(0),
};

function isEmptyProof(proof: ValidityProof): boolean {
    return (
        proof.a.every(a => a === 0) &&
        proof.b.every(b => b === 0) &&
        proof.c.every(c => c === 0)
    );
}

export function encodeApproveInstructionData(
    data: CompressedTokenInstructionDataApprove,
): Buffer {
    const buffer = Buffer.alloc(1000);

    const proofOption = data.proof ?? emptyProof;

    const len = CompressedTokenInstructionDataApproveLayout.encode(
        {
            ...data,
            proof: proofOption,
        },
        buffer,
    );

    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);

    const dataBuffer = buffer.subarray(0, len);

    return Buffer.concat([
        new Uint8Array(APPROVE_DISCRIMINATOR),
        new Uint8Array(lengthBuffer),
        new Uint8Array(dataBuffer),
    ]);
}

export function decodeApproveInstructionData(
    buffer: Buffer,
): CompressedTokenInstructionDataApprove {
    const data = CompressedTokenInstructionDataApproveLayout.decode(
        buffer.subarray(APPROVE_DISCRIMINATOR.length),
    ) as CompressedTokenInstructionDataApprove;
    return {
        ...data,
        proof: isEmptyProof(data.proof!) ? null : data.proof!,
    };
}

export function encodeRevokeInstructionData(
    data: CompressedTokenInstructionDataRevoke,
): Buffer {
    const buffer = Buffer.alloc(1000);

    const proofOption = data.proof ?? emptyProof;

    const len = CompressedTokenInstructionDataRevokeLayout.encode(
        {
            ...data,
            proof: proofOption,
        },
        buffer,
    );

    const lengthBuffer = Buffer.alloc(4);
    lengthBuffer.writeUInt32LE(len, 0);

    const dataBuffer = buffer.subarray(0, len);

    return Buffer.concat([
        new Uint8Array(REVOKE_DISCRIMINATOR),
        new Uint8Array(lengthBuffer),
        new Uint8Array(dataBuffer),
    ]);
}

export function decodeRevokeInstructionData(
    buffer: Buffer,
): CompressedTokenInstructionDataRevoke {
    const data = CompressedTokenInstructionDataRevokeLayout.decode(
        buffer.subarray(REVOKE_DISCRIMINATOR.length),
    ) as CompressedTokenInstructionDataRevoke;
    return {
        ...data,
        proof: isEmptyProof(data.proof!) ? null : data.proof!,
    };
}
