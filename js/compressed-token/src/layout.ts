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
import BN from 'bn.js';
import {
    CompressedCpiContext,
    CompressedTokenInstructionDataTransfer,
} from './types';

export const CREATE_TOKEN_POOL_DISCRIMINATOR = Buffer.from([
    23, 169, 27, 122, 147, 169, 209, 152,
]);
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
            option(QueueIndexLayout, 'QueueIndex'),
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

export const compressSplTokenAccountInstructionDataLayout = struct([
    publicKey('owner'),
    option(u64(), 'remainingAmount'),
    option(CpiContextLayout, 'cpiContext'),
]);

const MINT_TO_DISCRIMINATOR = Buffer.from([
    241, 34, 48, 186, 37, 179, 123, 192,
]);
export const TRANSFER_DISCRIMINATOR = Buffer.from([
    163, 52, 200, 231, 140, 3, 69, 186,
]);
export function encodeMintToInstructionData(
    recipients: PublicKey[],
    amounts: BN[],
    lamports: BN | null,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = mintToLayout.encode(
        {
            recipients,
            amounts,
            lamports,
        },
        buffer,
    );
    return Buffer.concat([MINT_TO_DISCRIMINATOR, buffer.slice(0, len)]);
}

export const COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR = Buffer.from([
    112, 230, 105, 101, 145, 202, 157, 97,
]);

export function encodeCompressSplTokenAccountInstructionData(
    owner: PublicKey,
    remainingAmount: BN | null,
    cpiContext: CompressedCpiContext | null,
): Buffer {
    const buffer = Buffer.alloc(1000);
    const len = compressSplTokenAccountInstructionDataLayout.encode(
        {
            owner,
            remainingAmount,
            cpiContext,
        },
        buffer,
    );
    return Buffer.concat([
        COMPRESS_SPL_TOKEN_ACCOUNT_DISCRIMINATOR,
        buffer.slice(0, len),
    ]);
}

export function encodeCompressedTokenInstructionDataTransfer(
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

export type createTokenPoolAccountsLayoutParams = {
    feePayer: PublicKey;
    tokenPoolPda: PublicKey;
    systemProgram: PublicKey;
    mint: PublicKey;
    tokenProgram: PublicKey;
    cpiAuthorityPda: PublicKey;
};

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

export type mintToAccountsLayoutParams = {
    feePayer: PublicKey;
    authority: PublicKey;
    cpiAuthorityPda: PublicKey;
    mint: PublicKey;
    tokenPoolPda: PublicKey;
    tokenProgram: PublicKey;
    lightSystemProgram: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    merkleTree: PublicKey;
    selfProgram: PublicKey;
    systemProgram: PublicKey;
    solPoolPda: PublicKey | null;
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

export type compressSplTokenAccountAccountsLayoutParams = {
    feePayer: PublicKey;
    authority: PublicKey;
    cpiAuthorityPda: PublicKey;
    lightSystemProgram: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    selfProgram: PublicKey;
    tokenPoolPda?: PublicKey;
    compressOrDecompressTokenAccount?: PublicKey;
    tokenProgram?: PublicKey;
    systemProgram: PublicKey;
};

export const compressSplTokenAccountAccountsLayout = (
    accounts: compressSplTokenAccountAccountsLayoutParams,
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

export const compressSplTokenAccountArgsLayout = struct(
    [
        publicKey('owner'),
        option(u64(), 'remainingAmount'),
        option(struct([], 'CompressedCpiContext'), 'cpiContext'),
    ],
    'compressSplTokenAccountArgs',
);

export interface transferAccountsLayoutParams {
    feePayer: PublicKey;
    authority: PublicKey;
    cpiAuthorityPda: PublicKey;
    lightSystemProgram: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    selfProgram: PublicKey;
    tokenPoolPda?: PublicKey;
    compressOrDecompressTokenAccount?: PublicKey;
    tokenProgram?: PublicKey;
    systemProgram: PublicKey;
}

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

export type compressSplTokenAccountLayoutParams = {
    feePayer: PublicKey;
    authority: PublicKey;
    cpiAuthorityPda: PublicKey;
    lightSystemProgram: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    accountCompressionProgram: PublicKey;
    selfProgram: PublicKey;
    tokenPoolPda?: PublicKey;
    compressOrDecompressTokenAccount?: PublicKey;
    tokenProgram?: PublicKey;
    systemProgram: PublicKey;
};

export const compressSplTokenAccountLayout = (
    accounts: compressSplTokenAccountLayoutParams,
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
};

export type approveAccountsLayoutParams = {
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

export type revokeAccountsLayoutParams = approveAccountsLayoutParams;
export const revokeAccountsLayout = approveAccountsLayout;

export type freezeAccountsLayoutParams = {
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
    mint: PublicKey;
};

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

export type thawAccountsLayoutParams = freezeAccountsLayoutParams;
export const thawAccountsLayout = freezeAccountsLayout;
