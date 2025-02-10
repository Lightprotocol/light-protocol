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
import { Buffer } from 'buffer';
import { AccountMeta, PublicKey } from '@solana/web3.js';
import { CompressedTokenProgram } from './program';
import {
    CompressedTokenInstructionDataTransfer,
    CompressSplTokenAccountInstructionData,
    CompressV2InstructionData,
    MintToInstructionData,
} from './types';
import {
    COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR,
    COMPRESS_V2_DISCRIMINATOR,
    MINT_TO_DISCRIMINATOR,
    TRANSFER_DISCRIMINATOR,
} from './constants';

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

const QueueIndexLayout = struct([u8('queueId'), u16('index')]);

const InputTokenDataWithContextLayout = struct([
    u64('amount'),
    option(u8(), 'delegateIndex'),
    struct(
        [
            u8('merkleTreePubkeyIndex'),
            u8('nullifierQueuePubkeyIndex'),
            u32('leafIndex'),
            option(QueueIndexLayout, 'queueIndex'),
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

export const compressV2InstructionDataLayout = struct([
    vec(publicKey(), 'publicKeys'),
    u64('amount'),
    option(u64(), 'lamports'),
]);

export const compressSplTokenAccountInstructionDataLayout = struct([
    publicKey('owner'),
    option(u64(), 'remainingAmount'),
    option(CpiContextLayout, 'cpiContext'),
]);

export function encodeCompressV2InstructionData(
    data: CompressV2InstructionData,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = compressV2InstructionDataLayout.encode(
        {
            publicKeys: data.publicKeys,
            amount: data.amount,
            lamports: data.lamports,
        },
        buffer,
    );

    return Buffer.concat([COMPRESS_V2_DISCRIMINATOR, buffer.slice(0, len)]);
}

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

    return Buffer.concat([MINT_TO_DISCRIMINATOR, buffer.slice(0, len)]);
}

export function decodeMintToInstructionData(
    buffer: Buffer,
): MintToInstructionData {
    const data: any = mintToLayout.decode(
        buffer.slice(MINT_TO_DISCRIMINATOR.length),
    );
    return {
        recipients: data.recipients,
        amounts: data.amounts,
        lamports: data.lamports,
    };
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
        COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR,
        buffer.slice(0, len),
    ]);
}

export function decodeCompressSplTokenAccountInstructionData(
    buffer: Buffer,
): CompressSplTokenAccountInstructionData {
    const data: any = compressSplTokenAccountInstructionDataLayout.decode(
        buffer.slice(COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR.length),
    );
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

    return Buffer.concat([
        TRANSFER_DISCRIMINATOR,
        lengthBuffer,
        buffer.slice(0, len),
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

export const compressV2AccountsLayout = (
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
        { pubkey: mint, isSigner: false, isWritable: false },
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

// TODO: use this layout for approve/revoke/freeze/thaw once we add them
// export const approveAccountsLayout = (
//     accounts: approveAccountsLayoutParams,
// ): AccountMeta[] => {
//     const {
//         feePayer,
//         authority,
//         cpiAuthorityPda,
//         lightSystemProgram,
//         registeredProgramPda,
//         noopProgram,
//         accountCompressionAuthority,
//         accountCompressionProgram,
//         selfProgram,
//         systemProgram,
//     } = accounts;

//     return [
//         { pubkey: feePayer, isSigner: true, isWritable: true },
//         { pubkey: authority, isSigner: true, isWritable: false },
//         { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
//         { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
//         { pubkey: registeredProgramPda, isSigner: false, isWritable: false },
//         { pubkey: noopProgram, isSigner: false, isWritable: false },
//         {
//             pubkey: accountCompressionAuthority,
//             isSigner: false,
//             isWritable: false,
//         },
//         {
//             pubkey: accountCompressionProgram,
//             isSigner: false,
//             isWritable: false,
//         },
//         { pubkey: selfProgram, isSigner: false, isWritable: false },
//         { pubkey: systemProgram, isSigner: false, isWritable: false },
//     ];
// };

// export const revokeAccountsLayout = approveAccountsLayout;

// export const freezeAccountsLayout = (
//     accounts: freezeAccountsLayoutParams,
// ): AccountMeta[] => {
//     const {
//         feePayer,
//         authority,
//         cpiAuthorityPda,
//         lightSystemProgram,
//         registeredProgramPda,
//         noopProgram,
//         accountCompressionAuthority,
//         accountCompressionProgram,
//         selfProgram,
//         systemProgram,
//         mint,
//     } = accounts;

//     return [
//         { pubkey: feePayer, isSigner: true, isWritable: true },
//         { pubkey: authority, isSigner: true, isWritable: false },
//         { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
//         { pubkey: lightSystemProgram, isSigner: false, isWritable: false },
//         { pubkey: registeredProgramPda, isSigner: false, isWritable: false },
//         { pubkey: noopProgram, isSigner: false, isWritable: false },
//         {
//             pubkey: accountCompressionAuthority,
//             isSigner: false,
//             isWritable: false,
//         },
//         {
//             pubkey: accountCompressionProgram,
//             isSigner: false,
//             isWritable: false,
//         },
//         { pubkey: selfProgram, isSigner: false, isWritable: false },
//         { pubkey: systemProgram, isSigner: false, isWritable: false },
//         { pubkey: mint, isSigner: false, isWritable: false },
//     ];
// };

// export const thawAccountsLayout = freezeAccountsLayout;
