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
} from '@coral-xyz/borsh';
import { Buffer } from 'buffer';
import { bn } from '@lightprotocol/stateless.js';

// Transfer2 discriminator = 101
export const TRANSFER2_DISCRIMINATOR = Buffer.from([101]);

// CompressionMode enum values
export const COMPRESSION_MODE_COMPRESS = 0;
export const COMPRESSION_MODE_DECOMPRESS = 1;
export const COMPRESSION_MODE_COMPRESS_AND_CLOSE = 2;

/**
 * Compression struct for Transfer2 instruction
 */
export interface Compression {
    mode: number;
    amount: bigint;
    mint: number;
    sourceOrRecipient: number;
    authority: number;
    poolAccountIndex: number;
    poolIndex: number;
    bump: number;
    decimals: number;
}

/**
 * Packed merkle context for compressed accounts
 */
export interface PackedMerkleContext {
    merkleTreePubkeyIndex: number;
    queuePubkeyIndex: number;
    leafIndex: number;
    proveByIndex: boolean;
}

/**
 * Input token data with context for Transfer2
 */
export interface MultiInputTokenDataWithContext {
    owner: number;
    amount: bigint;
    hasDelegate: boolean;
    delegate: number;
    mint: number;
    version: number;
    merkleContext: PackedMerkleContext;
    rootIndex: number;
}

/**
 * Output token data for Transfer2
 */
export interface MultiTokenTransferOutputData {
    owner: number;
    amount: bigint;
    hasDelegate: boolean;
    delegate: number;
    mint: number;
    version: number;
}

/**
 * CPI context for Transfer2
 */
export interface CompressedCpiContext {
    setContext: boolean;
    firstSetContext: boolean;
    cpiContextAccountIndex: number;
}

/**
 * Full Transfer2 instruction data
 */
export interface Transfer2InstructionData {
    withTransactionHash: boolean;
    withLamportsChangeAccountMerkleTreeIndex: boolean;
    lamportsChangeAccountMerkleTreeIndex: number;
    lamportsChangeAccountOwnerIndex: number;
    outputQueue: number;
    maxTopUp: number;
    cpiContext: CompressedCpiContext | null;
    compressions: Compression[] | null;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    inTokenData: MultiInputTokenDataWithContext[];
    outTokenData: MultiTokenTransferOutputData[];
    inLamports: bigint[] | null;
    outLamports: bigint[] | null;
    inTlv: number[][] | null;
    outTlv: number[][] | null;
}

// Borsh layouts
const CompressionLayout = struct([
    u8('mode'),
    u64('amount'),
    u8('mint'),
    u8('sourceOrRecipient'),
    u8('authority'),
    u8('poolAccountIndex'),
    u8('poolIndex'),
    u8('bump'),
    u8('decimals'),
]);

const PackedMerkleContextLayout = struct([
    u8('merkleTreePubkeyIndex'),
    u8('queuePubkeyIndex'),
    u32('leafIndex'),
    bool('proveByIndex'),
]);

const MultiInputTokenDataWithContextLayout = struct([
    u8('owner'),
    u64('amount'),
    bool('hasDelegate'),
    u8('delegate'),
    u8('mint'),
    u8('version'),
    PackedMerkleContextLayout.replicate('merkleContext'),
    u16('rootIndex'),
]);

const MultiTokenTransferOutputDataLayout = struct([
    u8('owner'),
    u64('amount'),
    bool('hasDelegate'),
    u8('delegate'),
    u8('mint'),
    u8('version'),
]);

const CompressedCpiContextLayout = struct([
    bool('setContext'),
    bool('firstSetContext'),
    u8('cpiContextAccountIndex'),
]);

const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

const Transfer2InstructionDataLayout = struct([
    bool('withTransactionHash'),
    bool('withLamportsChangeAccountMerkleTreeIndex'),
    u8('lamportsChangeAccountMerkleTreeIndex'),
    u8('lamportsChangeAccountOwnerIndex'),
    u8('outputQueue'),
    u16('maxTopUp'),
    option(CompressedCpiContextLayout, 'cpiContext'),
    option(vec(CompressionLayout), 'compressions'),
    option(CompressedProofLayout, 'proof'),
    vec(MultiInputTokenDataWithContextLayout, 'inTokenData'),
    vec(MultiTokenTransferOutputDataLayout, 'outTokenData'),
    option(vec(u64()), 'inLamports'),
    option(vec(u64()), 'outLamports'),
    option(vec(vec(u8())), 'inTlv'),
    option(vec(vec(u8())), 'outTlv'),
]);

/**
 * Encode Transfer2 instruction data using Borsh
 */
export function encodeTransfer2InstructionData(
    data: Transfer2InstructionData,
): Buffer {
    // Convert bigint values to BN for Borsh encoding
    const encodableData = {
        ...data,
        compressions:
            data.compressions?.map(c => ({
                ...c,
                amount: bn(c.amount.toString()),
            })) ?? null,
        inTokenData: data.inTokenData.map(t => ({
            ...t,
            amount: bn(t.amount.toString()),
        })),
        outTokenData: data.outTokenData.map(t => ({
            ...t,
            amount: bn(t.amount.toString()),
        })),
        inLamports: data.inLamports?.map(v => bn(v.toString())) ?? null,
        outLamports: data.outLamports?.map(v => bn(v.toString())) ?? null,
    };

    const buffer = Buffer.alloc(2000); // Allocate enough space
    const len = Transfer2InstructionDataLayout.encode(encodableData, buffer);
    return Buffer.concat([TRANSFER2_DISCRIMINATOR, buffer.subarray(0, len)]);
}

/**
 * Create a compression struct for wrapping SPL tokens to c-token
 * (compress from SPL ATA)
 */
export function createCompressSpl(
    amount: bigint,
    mintIndex: number,
    sourceIndex: number,
    authorityIndex: number,
    poolAccountIndex: number,
    poolIndex: number,
    bump: number,
    decimals: number,
): Compression {
    return {
        mode: COMPRESSION_MODE_COMPRESS,
        amount,
        mint: mintIndex,
        sourceOrRecipient: sourceIndex,
        authority: authorityIndex,
        poolAccountIndex,
        poolIndex,
        bump,
        decimals,
    };
}

/**
 * Create a compression struct for decompressing to c-token ATA
 * @param amount - Amount to decompress
 * @param mintIndex - Index of mint in packed accounts
 * @param recipientIndex - Index of recipient c-token account in packed accounts
 * @param tokenProgramIndex - Index of c-token program in packed accounts (for CPI)
 */
export function createDecompressCtoken(
    amount: bigint,
    mintIndex: number,
    recipientIndex: number,
    tokenProgramIndex?: number,
): Compression {
    return {
        mode: COMPRESSION_MODE_DECOMPRESS,
        amount,
        mint: mintIndex,
        sourceOrRecipient: recipientIndex,
        authority: 0,
        poolAccountIndex: tokenProgramIndex ?? 0,
        poolIndex: 0,
        bump: 0,
        decimals: 0,
    };
}

/**
 * Create a compression struct for compressing c-token (burn from c-token ATA)
 * Used in unwrap flow: c-token ATA -> pool -> SPL ATA
 * @param amount - Amount to compress (burn from c-token)
 * @param mintIndex - Index of mint in packed accounts
 * @param sourceIndex - Index of source c-token account in packed accounts
 * @param authorityIndex - Index of authority/owner in packed accounts (must sign)
 * @param tokenProgramIndex - Index of c-token program in packed accounts (for CPI)
 */
export function createCompressCtoken(
    amount: bigint,
    mintIndex: number,
    sourceIndex: number,
    authorityIndex: number,
    tokenProgramIndex?: number,
): Compression {
    return {
        mode: COMPRESSION_MODE_COMPRESS,
        amount,
        mint: mintIndex,
        sourceOrRecipient: sourceIndex,
        authority: authorityIndex,
        poolAccountIndex: tokenProgramIndex ?? 0,
        poolIndex: 0,
        bump: 0,
        decimals: 0,
    };
}

/**
 * Create a compression struct for decompressing SPL tokens
 */
export function createDecompressSpl(
    amount: bigint,
    mintIndex: number,
    recipientIndex: number,
    poolAccountIndex: number,
    poolIndex: number,
    bump: number,
    decimals: number,
): Compression {
    return {
        mode: COMPRESSION_MODE_DECOMPRESS,
        amount,
        mint: mintIndex,
        sourceOrRecipient: recipientIndex,
        authority: 0,
        poolAccountIndex,
        poolIndex,
        bump,
        decimals,
    };
}
