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
import { PublicKey } from '@solana/web3.js';
import { CompressionInfo, RentConfig } from './layout-mint';
import { AdditionalMetadata } from './layout-mint-action';

// Transfer2 discriminator = 101
export const TRANSFER2_DISCRIMINATOR = Buffer.from([101]);

// Extension discriminant values (matching Rust enum)
export const EXTENSION_DISCRIMINANT_TOKEN_METADATA = 19;
export const EXTENSION_DISCRIMINANT_COMPRESSED_ONLY = 31;
export const EXTENSION_DISCRIMINANT_COMPRESSIBLE = 32;

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
 * Token metadata extension instruction data for Transfer2 TLV
 */
export interface Transfer2TokenMetadata {
    updateAuthority: PublicKey | null;
    name: Uint8Array;
    symbol: Uint8Array;
    uri: Uint8Array;
    additionalMetadata: AdditionalMetadata[] | null;
}

/**
 * CompressedOnly extension instruction data for Transfer2 TLV
 */
export interface Transfer2CompressedOnly {
    delegatedAmount: bigint;
    withheldTransferFee: bigint;
    isFrozen: boolean;
    compressionIndex: number;
    isAta: boolean;
    bump: number;
    ownerIndex: number;
}

/**
 * Extension instruction data types for Transfer2 in_tlv/out_tlv
 */
export type Transfer2ExtensionData =
    | { type: 'TokenMetadata'; data: Transfer2TokenMetadata }
    | { type: 'CompressedOnly'; data: Transfer2CompressedOnly }
    | { type: 'Compressible'; data: CompressionInfo };

/**
 * Full Transfer2 instruction data
 *
 * Note on `decimals` field in Compression:
 * - For SPL compress/decompress: actual token decimals
 * - For CompressAndClose mode: used as `rent_sponsor_is_signer` flag
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
    /** Extensions for input light-token accounts (one array per input account) */
    inTlv: Transfer2ExtensionData[][] | null;
    /** Extensions for output light-token accounts (one array per output account) */
    outTlv: Transfer2ExtensionData[][] | null;
}

// Borsh layouts for extension data
const AdditionalMetadataLayout = struct([vec(u8(), 'key'), vec(u8(), 'value')]);

const TokenMetadataInstructionDataLayout = struct([
    option(array(u8(), 32), 'updateAuthority'),
    vec(u8(), 'name'),
    vec(u8(), 'symbol'),
    vec(u8(), 'uri'),
    option(vec(AdditionalMetadataLayout), 'additionalMetadata'),
]);

const CompressedOnlyExtensionInstructionDataLayout = struct([
    u64('delegatedAmount'),
    u64('withheldTransferFee'),
    bool('isFrozen'),
    u8('compressionIndex'),
    bool('isAta'),
    u8('bump'),
    u8('ownerIndex'),
]);

const CompressToPubkeyLayout = struct([
    u8('bump'),
    array(u8(), 32, 'programId'),
    vec(vec(u8()), 'seeds'),
]);

const RentConfigLayout = struct([
    u16('baseRent'),
    u16('compressionCost'),
    u8('lamportsPerBytePerEpoch'),
    u8('maxFundedEpochs'),
    u16('maxTopUp'),
]);

const CompressionInfoLayout = struct([
    u16('configAccountVersion'),
    u8('compressToPubkey'),
    u8('accountVersion'),
    u32('lamportsPerWrite'),
    array(u8(), 32, 'compressionAuthority'),
    array(u8(), 32, 'rentSponsor'),
    u64('lastClaimedSlot'),
    u32('rentExemptionPaid'),
    u32('reserved'),
    RentConfigLayout.replicate('rentConfig'),
]);

/**
 * Serialize a single Transfer2ExtensionData to bytes
 */
function serializeExtensionInstructionData(
    ext: Transfer2ExtensionData,
): Uint8Array {
    const buffer = Buffer.alloc(1024);
    let offset = 0;

    // Write discriminant
    if (ext.type === 'TokenMetadata') {
        buffer.writeUInt8(EXTENSION_DISCRIMINANT_TOKEN_METADATA, offset);
        offset += 1;
        const data = {
            updateAuthority: ext.data.updateAuthority
                ? Array.from(ext.data.updateAuthority.toBytes())
                : null,
            name: Array.from(ext.data.name),
            symbol: Array.from(ext.data.symbol),
            uri: Array.from(ext.data.uri),
            additionalMetadata: ext.data.additionalMetadata
                ? ext.data.additionalMetadata.map(m => ({
                      key: Array.from(m.key),
                      value: Array.from(m.value),
                  }))
                : null,
        };
        offset += TokenMetadataInstructionDataLayout.encode(
            data,
            buffer,
            offset,
        );
    } else if (ext.type === 'CompressedOnly') {
        buffer.writeUInt8(EXTENSION_DISCRIMINANT_COMPRESSED_ONLY, offset);
        offset += 1;
        const data = {
            delegatedAmount: bn(ext.data.delegatedAmount.toString()),
            withheldTransferFee: bn(ext.data.withheldTransferFee.toString()),
            isFrozen: ext.data.isFrozen,
            compressionIndex: ext.data.compressionIndex,
            isAta: ext.data.isAta,
            bump: ext.data.bump,
            ownerIndex: ext.data.ownerIndex,
        };
        offset += CompressedOnlyExtensionInstructionDataLayout.encode(
            data,
            buffer,
            offset,
        );
    } else if (ext.type === 'Compressible') {
        buffer.writeUInt8(EXTENSION_DISCRIMINANT_COMPRESSIBLE, offset);
        offset += 1;
        const data = {
            configAccountVersion: ext.data.configAccountVersion,
            compressToPubkey: ext.data.compressToPubkey,
            accountVersion: ext.data.accountVersion,
            lamportsPerWrite: ext.data.lamportsPerWrite,
            compressionAuthority: Array.from(
                ext.data.compressionAuthority.toBytes(),
            ),
            rentSponsor: Array.from(ext.data.rentSponsor.toBytes()),
            lastClaimedSlot: bn(ext.data.lastClaimedSlot.toString()),
            rentExemptionPaid: ext.data.rentExemptionPaid,
            reserved: ext.data.reserved,
            rentConfig: ext.data.rentConfig,
        };
        offset += CompressionInfoLayout.encode(data, buffer, offset);
    }

    return buffer.subarray(0, offset);
}

/**
 * Serialize Vec<Vec<Transfer2ExtensionData>> to bytes for Borsh
 */
function serializeExtensionTlv(
    tlv: Transfer2ExtensionData[][] | null,
): Uint8Array | null {
    if (tlv === null) {
        return null;
    }

    const chunks: Uint8Array[] = [];

    // Write outer vec length (4 bytes, little-endian)
    const outerLenBuf = Buffer.alloc(4);
    outerLenBuf.writeUInt32LE(tlv.length, 0);
    chunks.push(outerLenBuf);

    for (const innerVec of tlv) {
        // Write inner vec length (4 bytes, little-endian)
        const innerLenBuf = Buffer.alloc(4);
        innerLenBuf.writeUInt32LE(innerVec.length, 0);
        chunks.push(innerLenBuf);

        for (const ext of innerVec) {
            chunks.push(serializeExtensionInstructionData(ext));
        }
    }

    return Buffer.concat(chunks);
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

// Layout without TLV fields - we'll serialize those manually
const Transfer2InstructionDataBaseLayout = struct([
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
]);

/**
 * Encode Transfer2 instruction data using Borsh
 */
export function encodeTransfer2InstructionData(
    data: Transfer2InstructionData,
): Buffer {
    // Convert bigint values to BN for Borsh encoding
    const baseData = {
        withTransactionHash: data.withTransactionHash,
        withLamportsChangeAccountMerkleTreeIndex:
            data.withLamportsChangeAccountMerkleTreeIndex,
        lamportsChangeAccountMerkleTreeIndex:
            data.lamportsChangeAccountMerkleTreeIndex,
        lamportsChangeAccountOwnerIndex: data.lamportsChangeAccountOwnerIndex,
        outputQueue: data.outputQueue,
        maxTopUp: data.maxTopUp,
        cpiContext: data.cpiContext,
        compressions:
            data.compressions?.map(c => ({
                ...c,
                amount: bn(c.amount.toString()),
            })) ?? null,
        proof: data.proof,
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

    // Encode base layout
    const baseBuffer = Buffer.alloc(4000);
    const baseLen = Transfer2InstructionDataBaseLayout.encode(
        baseData,
        baseBuffer,
    );

    // Manually serialize TLV fields
    const chunks: Buffer[] = [
        TRANSFER2_DISCRIMINATOR,
        baseBuffer.subarray(0, baseLen),
    ];

    // Serialize inTlv as Option<Vec<Vec<ExtensionInstructionData>>>
    if (data.inTlv === null) {
        // Option::None = 0
        chunks.push(Buffer.from([0]));
    } else {
        // Option::Some = 1
        chunks.push(Buffer.from([1]));
        const serialized = serializeExtensionTlv(data.inTlv);
        if (serialized) {
            chunks.push(Buffer.from(serialized));
        }
    }

    // Serialize outTlv as Option<Vec<Vec<ExtensionInstructionData>>>
    if (data.outTlv === null) {
        // Option::None = 0
        chunks.push(Buffer.from([0]));
    } else {
        // Option::Some = 1
        chunks.push(Buffer.from([1]));
        const serialized = serializeExtensionTlv(data.outTlv);
        if (serialized) {
            chunks.push(Buffer.from(serialized));
        }
    }

    return Buffer.concat(chunks);
}

/**
 * Create a compression struct for wrapping SPL tokens to light-token
 * (compress from SPL associated token account)
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
 * Create a compression struct for decompressing to light-token associated token account
 * @param amount - Amount to decompress
 * @param mintIndex - Index of mint in packed accounts
 * @param recipientIndex - Index of recipient light-token account in packed accounts
 * @param tokenProgramIndex - Index of light-token program in packed accounts (for CPI)
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
 * Create a compression struct for compressing light-token (burn from light-token associated token account)
 * Used in unwrap flow: light-token associated token account -> pool -> SPL associated token account
 * @param amount - Amount to compress (burn from light-token)
 * @param mintIndex - Index of mint in packed accounts
 * @param sourceIndex - Index of source light-token account in packed accounts
 * @param authorityIndex - Index of authority/owner in packed accounts (must sign)
 * @param tokenProgramIndex - Index of light-token program in packed accounts (for CPI)
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
