import { MINT_SIZE, MintLayout } from '@solana/spl-token';
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { struct, u8, u32 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import {
    struct as borshStruct,
    option,
    vec,
    vecU8,
    publicKey as borshPublicKey,
} from '@coral-xyz/borsh';

/**
 * SPL-compatible base mint structure
 */
export interface BaseMint {
    /** Optional authority used to mint new tokens */
    mintAuthority: PublicKey | null;
    /** Total supply of tokens */
    supply: bigint;
    /** Number of base 10 digits to the right of the decimal place */
    decimals: number;
    /** Is initialized - for SPL compatibility */
    isInitialized: boolean;
    /** Optional authority to freeze token accounts */
    freezeAuthority: PublicKey | null;
}

/**
 * Compressed mint context (protocol version, SPL mint reference)
 */
export interface MintContext {
    /** Protocol version for upgradability */
    version: number;
    /** Whether the compressed mint is decompressed to a CMint Solana account */
    cmintDecompressed: boolean;
    /** PDA of the associated SPL mint */
    splMint: PublicKey;
}

/**
 * Raw extension data as stored on-chain
 */
export interface MintExtension {
    extensionType: number;
    data: Uint8Array;
}

/**
 * Parsed token metadata matching on-chain TokenMetadata extension.
 * Fields: updateAuthority, mint, name, symbol, uri, additionalMetadata
 */
export interface TokenMetadata {
    /** Authority that can update metadata (None if zero pubkey) */
    updateAuthority?: PublicKey | null;
    /** Associated mint pubkey */
    mint: PublicKey;
    /** Token name */
    name: string;
    /** Token symbol */
    symbol: string;
    /** URI pointing to off-chain metadata JSON */
    uri: string;
    /** Additional key-value metadata pairs */
    additionalMetadata?: { key: string; value: string }[];
}

/**
 * Borsh layout for TokenMetadata extension data
 * Format: updateAuthority (32) + mint (32) + name + symbol + uri + additional_metadata
 */
export const TokenMetadataLayout = borshStruct([
    borshPublicKey('updateAuthority'),
    borshPublicKey('mint'),
    vecU8('name'),
    vecU8('symbol'),
    vecU8('uri'),
    vec(borshStruct([vecU8('key'), vecU8('value')]), 'additionalMetadata'),
]);

/**
 * Complete compressed mint structure (raw format)
 */
export interface CompressedMint {
    base: BaseMint;
    mintContext: MintContext;
    extensions: MintExtension[] | null;
}

/** MintContext as stored by the program */
export interface RawMintContext {
    version: number;
    cmintDecompressed: number; // bool as u8
    splMint: PublicKey;
}

/** Buffer layout for de/serializing MintContext */
export const MintContextLayout = struct<RawMintContext>([
    u8('version'),
    u8('cmintDecompressed'),
    publicKey('splMint'),
]);

/** Byte length of MintContext */
export const MINT_CONTEXT_SIZE = MintContextLayout.span; // 34 bytes

/**
 * Calculate the byte length of a TokenMetadata extension from buffer.
 * Format: updateAuthority (32) + mint (32) + name (4+len) + symbol (4+len) + uri (4+len) + additional (4 + items)
 */
function getTokenMetadataByteLength(
    buffer: Buffer,
    startOffset: number,
): number {
    let offset = startOffset;

    // updateAuthority: 32 bytes
    offset += 32;
    // mint: 32 bytes
    offset += 32;

    // name: Vec<u8>
    const nameLen = buffer.readUInt32LE(offset);
    offset += 4 + nameLen;

    // symbol: Vec<u8>
    const symbolLen = buffer.readUInt32LE(offset);
    offset += 4 + symbolLen;

    // uri: Vec<u8>
    const uriLen = buffer.readUInt32LE(offset);
    offset += 4 + uriLen;

    // additional_metadata: Vec<AdditionalMetadata>
    const additionalCount = buffer.readUInt32LE(offset);
    offset += 4;
    for (let i = 0; i < additionalCount; i++) {
        const keyLen = buffer.readUInt32LE(offset);
        offset += 4 + keyLen;
        const valueLen = buffer.readUInt32LE(offset);
        offset += 4 + valueLen;
    }

    return offset - startOffset;
}

/**
 * Get the byte length of an extension based on its type.
 * Returns the length of the extension data (excluding the 1-byte discriminant).
 */
function getExtensionByteLength(
    extensionType: number,
    buffer: Buffer,
    dataStartOffset: number,
): number {
    switch (extensionType) {
        case ExtensionType.TokenMetadata:
            return getTokenMetadataByteLength(buffer, dataStartOffset);
        default:
            // For unknown extensions, we can't determine the length
            // Return remaining buffer length as fallback
            return buffer.length - dataStartOffset;
    }
}

/**
 * Deserialize a compressed mint from buffer
 * Uses SPL's MintLayout for BaseMint and buffer-layout struct for context
 *
 * @param data - The raw account data buffer
 * @returns The deserialized CompressedMint
 */
export function deserializeMint(data: Buffer | Uint8Array): CompressedMint {
    const buffer = data instanceof Buffer ? data : Buffer.from(data);
    let offset = 0;

    // 1. Decode BaseMint using SPL's MintLayout (82 bytes)
    const rawMint = MintLayout.decode(buffer.slice(offset, offset + MINT_SIZE));
    offset += MINT_SIZE;

    // 2. Decode MintContext using our layout (34 bytes)
    const rawContext = MintContextLayout.decode(
        buffer.slice(offset, offset + MINT_CONTEXT_SIZE),
    );
    offset += MINT_CONTEXT_SIZE;

    // 3. Parse extensions: Option<Vec<ExtensionStruct>>
    // Borsh format: Option byte + Vec length + (discriminant + variant data) for each
    const hasExtensions = buffer.readUInt8(offset) === 1;
    offset += 1;

    let extensions: MintExtension[] | null = null;
    if (hasExtensions) {
        const vecLen = buffer.readUInt32LE(offset);
        offset += 4;

        extensions = [];
        for (let i = 0; i < vecLen; i++) {
            const extensionType = buffer.readUInt8(offset);
            offset += 1;

            // Calculate extension data length based on type
            const dataLength = getExtensionByteLength(
                extensionType,
                buffer,
                offset,
            );
            const extensionData = buffer.slice(offset, offset + dataLength);
            offset += dataLength;

            extensions.push({
                extensionType,
                data: extensionData,
            });
        }
    }

    // Convert raw types to our interface with proper null handling
    const baseMint: BaseMint = {
        mintAuthority:
            rawMint.mintAuthorityOption === 1 ? rawMint.mintAuthority : null,
        supply: rawMint.supply,
        decimals: rawMint.decimals,
        isInitialized: rawMint.isInitialized,
        freezeAuthority:
            rawMint.freezeAuthorityOption === 1
                ? rawMint.freezeAuthority
                : null,
    };

    const mintContext: MintContext = {
        version: rawContext.version,
        cmintDecompressed: rawContext.cmintDecompressed !== 0,
        splMint: rawContext.splMint,
    };

    const mint: CompressedMint = {
        base: baseMint,
        mintContext,
        extensions,
    };

    return mint;
}

/**
 * Serialize a CompressedMint to buffer
 * Uses SPL's MintLayout for BaseMint, helper functions for context/metadata
 *
 * @param mint - The CompressedMint to serialize
 * @returns The serialized buffer
 */
export function serializeMint(mint: CompressedMint): Buffer {
    const buffers: Buffer[] = [];

    // 1. Encode BaseMint using SPL's MintLayout (82 bytes)
    const baseMintBuffer = Buffer.alloc(MINT_SIZE);
    MintLayout.encode(
        {
            mintAuthorityOption: mint.base.mintAuthority ? 1 : 0,
            mintAuthority: mint.base.mintAuthority || new PublicKey(0),
            supply: mint.base.supply,
            decimals: mint.base.decimals,
            isInitialized: mint.base.isInitialized,
            freezeAuthorityOption: mint.base.freezeAuthority ? 1 : 0,
            freezeAuthority: mint.base.freezeAuthority || new PublicKey(0),
        },
        baseMintBuffer,
    );
    buffers.push(baseMintBuffer);

    // 2. Encode MintContext using our layout (34 bytes)
    const contextBuffer = Buffer.alloc(MINT_CONTEXT_SIZE);
    MintContextLayout.encode(
        {
            version: mint.mintContext.version,
            cmintDecompressed: mint.mintContext.cmintDecompressed ? 1 : 0,
            splMint: mint.mintContext.splMint,
        },
        contextBuffer,
    );
    buffers.push(contextBuffer);

    // 3. Encode extensions: Option<Vec<ExtensionStruct>>
    // Borsh format: Option byte + Vec length + (discriminant + variant data) for each
    // NOTE: No length prefix per extension - Borsh enums are discriminant + data directly
    if (mint.extensions && mint.extensions.length > 0) {
        buffers.push(Buffer.from([1])); // Some
        const vecLenBuf = Buffer.alloc(4);
        vecLenBuf.writeUInt32LE(mint.extensions.length);
        buffers.push(vecLenBuf);

        for (const ext of mint.extensions) {
            // Write discriminant (1 byte)
            buffers.push(Buffer.from([ext.extensionType]));
            // Write extension data directly (no length prefix - Borsh format)
            buffers.push(Buffer.from(ext.data));
        }
    } else {
        buffers.push(Buffer.from([0])); // None
    }

    return Buffer.concat(buffers);
}

/**
 * Extension type constants
 */
export enum ExtensionType {
    TokenMetadata = 19, // Name, symbol, uri
    // Add more extension types as needed
}

/**
 * Decode TokenMetadata from raw extension data using Borsh layout
 * Extension format: updateAuthority (32) + mint (32) + name (Vec) + symbol (Vec) + uri (Vec) + additional (Vec)
 */
export function decodeTokenMetadata(data: Uint8Array): TokenMetadata | null {
    try {
        const buffer = Buffer.from(data);
        // Minimum size: 32 (updateAuthority) + 32 (mint) + 4 (name len) + 4 (symbol len) + 4 (uri len) + 4 (additional len) = 80
        if (buffer.length < 80) {
            return null;
        }

        // Decode using Borsh layout
        const decoded = TokenMetadataLayout.decode(buffer) as {
            updateAuthority: PublicKey;
            mint: PublicKey;
            name: Buffer;
            symbol: Buffer;
            uri: Buffer;
            additionalMetadata: { key: Buffer; value: Buffer }[];
        };

        // Convert zero pubkey to undefined for updateAuthority
        const updateAuthorityBytes = decoded.updateAuthority.toBuffer();
        const isZero = updateAuthorityBytes.every((b: number) => b === 0);
        const updateAuthority = isZero ? undefined : decoded.updateAuthority;

        // Convert Buffer fields to strings
        const name = Buffer.from(decoded.name).toString('utf-8');
        const symbol = Buffer.from(decoded.symbol).toString('utf-8');
        const uri = Buffer.from(decoded.uri).toString('utf-8');

        // Convert additional metadata
        let additionalMetadata: { key: string; value: string }[] | undefined;
        if (
            decoded.additionalMetadata &&
            decoded.additionalMetadata.length > 0
        ) {
            additionalMetadata = decoded.additionalMetadata.map(item => ({
                key: Buffer.from(item.key).toString('utf-8'),
                value: Buffer.from(item.value).toString('utf-8'),
            }));
        }

        return {
            updateAuthority,
            mint: decoded.mint,
            name,
            symbol,
            uri,
            additionalMetadata,
        };
    } catch (e) {
        console.error('Failed to decode TokenMetadata:', e);
        return null;
    }
}

/**
 * Encode TokenMetadata to raw bytes using Borsh layout
 * @param metadata - TokenMetadata to encode
 * @returns Encoded buffer
 */
export function encodeTokenMetadata(metadata: TokenMetadata): Buffer {
    const buffer = Buffer.alloc(2000); // Allocate generous buffer

    // Use zero pubkey if updateAuthority is not provided
    const updateAuthority = metadata.updateAuthority || new PublicKey(0);

    const len = TokenMetadataLayout.encode(
        {
            updateAuthority,
            mint: metadata.mint,
            name: Buffer.from(metadata.name),
            symbol: Buffer.from(metadata.symbol),
            uri: Buffer.from(metadata.uri),
            additionalMetadata: metadata.additionalMetadata
                ? metadata.additionalMetadata.map(item => ({
                      key: Buffer.from(item.key),
                      value: Buffer.from(item.value),
                  }))
                : [],
        },
        buffer,
    );
    return buffer.subarray(0, len);
}

/**
 * @deprecated Use decodeTokenMetadata instead
 */
export const parseTokenMetadata = decodeTokenMetadata;

/**
 * Extract and parse TokenMetadata from extensions array
 * @param extensions - Array of raw extensions
 * @returns Parsed TokenMetadata or null if not found
 */
export function extractTokenMetadata(
    extensions: MintExtension[] | null,
): TokenMetadata | null {
    if (!extensions) return null;
    const metadataExt = extensions.find(
        ext => ext.extensionType === ExtensionType.TokenMetadata,
    );
    return metadataExt ? parseTokenMetadata(metadataExt.data) : null;
}

/**
 * Metadata portion of MintInstructionData
 * Used for instruction encoding when metadata extension is present
 */
export interface MintMetadataField {
    updateAuthority: PublicKey | null;
    name: string;
    symbol: string;
    uri: string;
}

/**
 * Flattened mint data structure for instruction encoding
 * This is the format expected by mint action instructions
 */
export interface MintInstructionData {
    supply: bigint;
    decimals: number;
    mintAuthority: PublicKey | null;
    freezeAuthority: PublicKey | null;
    splMint: PublicKey;
    cmintDecompressed: boolean;
    version: number;
    metadata?: MintMetadataField;
}

/**
 * MintInstructionData with required metadata field
 * Used for metadata update instructions where metadata must be present
 */
export interface MintInstructionDataWithMetadata extends MintInstructionData {
    metadata: MintMetadataField;
}

/**
 * Convert a deserialized CompressedMint to MintInstructionData format
 * This extracts and flattens the data structure for instruction encoding
 *
 * @param compressedMint - Deserialized CompressedMint from account data
 * @returns Flattened MintInstructionData for instruction encoding
 */
export function toMintInstructionData(
    compressedMint: CompressedMint,
): MintInstructionData {
    const { base, mintContext, extensions } = compressedMint;

    // Extract metadata from extensions if present
    const tokenMetadata = extractTokenMetadata(extensions);
    const metadata: MintMetadataField | undefined = tokenMetadata
        ? {
              updateAuthority: tokenMetadata.updateAuthority ?? null,
              name: tokenMetadata.name,
              symbol: tokenMetadata.symbol,
              uri: tokenMetadata.uri,
          }
        : undefined;

    return {
        supply: base.supply,
        decimals: base.decimals,
        mintAuthority: base.mintAuthority,
        freezeAuthority: base.freezeAuthority,
        splMint: mintContext.splMint,
        cmintDecompressed: mintContext.cmintDecompressed,
        version: mintContext.version,
        metadata,
    };
}

/**
 * Convert a deserialized CompressedMint to MintInstructionDataWithMetadata
 * Throws if the mint doesn't have metadata extension
 *
 * @param compressedMint - Deserialized CompressedMint from account data
 * @returns MintInstructionDataWithMetadata for metadata update instructions
 * @throws Error if metadata extension is not present
 */
export function toMintInstructionDataWithMetadata(
    compressedMint: CompressedMint,
): MintInstructionDataWithMetadata {
    const data = toMintInstructionData(compressedMint);

    if (!data.metadata) {
        throw new Error('CompressedMint does not have TokenMetadata extension');
    }

    return data as MintInstructionDataWithMetadata;
}
