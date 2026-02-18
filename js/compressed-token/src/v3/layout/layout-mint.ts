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
 * Light mint context (protocol version, SPL mint reference)
 */
export interface MintContext {
    /** Protocol version for upgradability */
    version: number;
    /** Whether the compressed light mint has been decompressed to a light mint account */
    cmintDecompressed: boolean;
    /** PDA of the associated SPL mint */
    splMint: PublicKey;
    /** Signer pubkey used to derive the mint PDA */
    mintSigner: Uint8Array;
    /** Bump seed for the mint PDA */
    bump: number;
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
 * Complete light mint structure (raw format)
 */
export interface CompressedMint {
    base: BaseMint;
    mintContext: MintContext;
    /** Reserved bytes for T22 layout compatibility */
    reserved: Uint8Array;
    /** Account type discriminator (1 = Mint) */
    accountType: number;
    /** Compression info embedded in mint */
    compression: CompressionInfo;
    extensions: MintExtension[] | null;
}

/** MintContext as stored by the program */
/**
 * Raw mint context for layout encoding (mintSigner and bump are encoded separately)
 */
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

/** Byte length of MintContext (excluding mintSigner and bump which are read separately) */
export const MINT_CONTEXT_SIZE = MintContextLayout.span; // 34 bytes

/** Additional bytes for mintSigner (32) + bump (1) */
export const MINT_SIGNER_SIZE = 32;
export const BUMP_SIZE = 1;

/** Reserved bytes for T22 layout compatibility (padding to reach byte 165) */
export const RESERVED_SIZE = 16;

/** Account type discriminator size */
export const ACCOUNT_TYPE_SIZE = 1;

/** Account type value for light mint */
export const ACCOUNT_TYPE_MINT = 1;

/**
 * Rent configuration for compressible accounts
 */
export interface RentConfig {
    /** Base rent constant per epoch */
    baseRent: number;
    /** Compression cost in lamports */
    compressionCost: number;
    /** Lamports per byte per epoch */
    lamportsPerBytePerEpoch: number;
    /** Maximum epochs that can be pre-funded */
    maxFundedEpochs: number;
    /** Maximum lamports for top-up operation */
    maxTopUp: number;
}

/** Byte length of RentConfig */
export const RENT_CONFIG_SIZE = 8; // 2 + 2 + 1 + 1 + 2

/**
 * Compression info embedded in light mint
 */
export interface CompressionInfo {
    /** Config account version (0 = uninitialized) */
    configAccountVersion: number;
    /** Whether to compress to pubkey instead of owner */
    compressToPubkey: number;
    /** Account version for hashing scheme */
    accountVersion: number;
    /** Lamports to top up per write */
    lamportsPerWrite: number;
    /** Authority that can compress the account */
    compressionAuthority: PublicKey;
    /** Recipient for rent on closure */
    rentSponsor: PublicKey;
    /** Last slot rent was claimed */
    lastClaimedSlot: bigint;
    /** Rent exemption lamports paid at account creation */
    rentExemptionPaid: number;
    /** Reserved for future use */
    reserved: number;
    /** Rent configuration */
    rentConfig: RentConfig;
}

/** Byte length of CompressionInfo */
export const COMPRESSION_INFO_SIZE = 96; // 2 + 1 + 1 + 4 + 32 + 32 + 8 + 4 + 4 + 8

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
 * Deserialize CompressionInfo from buffer at given offset
 * @returns Tuple of [CompressionInfo, bytesRead]
 */
function deserializeCompressionInfo(
    buffer: Buffer,
    offset: number,
): [CompressionInfo, number] {
    const startOffset = offset;

    const configAccountVersion = buffer.readUInt16LE(offset);
    offset += 2;

    const compressToPubkey = buffer.readUInt8(offset);
    offset += 1;

    const accountVersion = buffer.readUInt8(offset);
    offset += 1;

    const lamportsPerWrite = buffer.readUInt32LE(offset);
    offset += 4;

    const compressionAuthority = new PublicKey(
        buffer.slice(offset, offset + 32),
    );
    offset += 32;

    const rentSponsor = new PublicKey(buffer.slice(offset, offset + 32));
    offset += 32;

    const lastClaimedSlot = buffer.readBigUInt64LE(offset);
    offset += 8;

    // Read rent_exemption_paid (u32) and _reserved (u32)
    const rentExemptionPaid = buffer.readUInt32LE(offset);
    offset += 4;
    const reserved = buffer.readUInt32LE(offset);
    offset += 4;

    // Read RentConfig (8 bytes)
    const baseRent = buffer.readUInt16LE(offset);
    offset += 2;
    const compressionCost = buffer.readUInt16LE(offset);
    offset += 2;
    const lamportsPerBytePerEpoch = buffer.readUInt8(offset);
    offset += 1;
    const maxFundedEpochs = buffer.readUInt8(offset);
    offset += 1;
    const maxTopUp = buffer.readUInt16LE(offset);
    offset += 2;

    const rentConfig: RentConfig = {
        baseRent,
        compressionCost,
        lamportsPerBytePerEpoch,
        maxFundedEpochs,
        maxTopUp,
    };

    const compressionInfo: CompressionInfo = {
        configAccountVersion,
        compressToPubkey,
        accountVersion,
        lamportsPerWrite,
        compressionAuthority,
        rentSponsor,
        lastClaimedSlot,
        rentExemptionPaid,
        reserved,
        rentConfig,
    };

    return [compressionInfo, offset - startOffset];
}

/**
 * Deserialize a light mint from buffer
 * Uses SPL's MintLayout for BaseMint and buffer-layout struct for context
 *
 * @param data - The raw account data buffer
 * @returns The deserialized light mint
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

    // 2b. Read mintSigner (32 bytes) and bump (1 byte)
    const mintSigner = buffer.slice(offset, offset + MINT_SIGNER_SIZE);
    offset += MINT_SIGNER_SIZE;
    const bump = buffer.readUInt8(offset);
    offset += BUMP_SIZE;

    // 3. Read reserved bytes (49 bytes) for T22 compatibility
    const reserved = buffer.slice(offset, offset + RESERVED_SIZE);
    offset += RESERVED_SIZE;

    // 4. Read account_type discriminator (1 byte)
    const accountType = buffer.readUInt8(offset);
    offset += ACCOUNT_TYPE_SIZE;

    // 5. Read CompressionInfo (88 bytes)
    const [compression, compressionBytesRead] = deserializeCompressionInfo(
        buffer,
        offset,
    );
    offset += compressionBytesRead;

    // 6. Parse extensions: Option<Vec<ExtensionStruct>>
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
        mintSigner,
        bump,
    };

    const mint: CompressedMint = {
        base: baseMint,
        mintContext,
        reserved,
        accountType,
        compression,
        extensions,
    };

    return mint;
}

/**
 * Serialize CompressionInfo to buffer
 */
function serializeCompressionInfo(compression: CompressionInfo): Buffer {
    const buffer = Buffer.alloc(COMPRESSION_INFO_SIZE);
    let offset = 0;

    buffer.writeUInt16LE(compression.configAccountVersion, offset);
    offset += 2;

    buffer.writeUInt8(compression.compressToPubkey, offset);
    offset += 1;

    buffer.writeUInt8(compression.accountVersion, offset);
    offset += 1;

    buffer.writeUInt32LE(compression.lamportsPerWrite, offset);
    offset += 4;

    compression.compressionAuthority.toBuffer().copy(buffer, offset);
    offset += 32;

    compression.rentSponsor.toBuffer().copy(buffer, offset);
    offset += 32;

    buffer.writeBigUInt64LE(compression.lastClaimedSlot, offset);
    offset += 8;

    // Write rent_exemption_paid (u32) and _reserved (u32)
    buffer.writeUInt32LE(compression.rentExemptionPaid, offset);
    offset += 4;
    buffer.writeUInt32LE(compression.reserved, offset);
    offset += 4;

    // Write RentConfig (8 bytes)
    buffer.writeUInt16LE(compression.rentConfig.baseRent, offset);
    offset += 2;
    buffer.writeUInt16LE(compression.rentConfig.compressionCost, offset);
    offset += 2;
    buffer.writeUInt8(compression.rentConfig.lamportsPerBytePerEpoch, offset);
    offset += 1;
    buffer.writeUInt8(compression.rentConfig.maxFundedEpochs, offset);
    offset += 1;
    buffer.writeUInt16LE(compression.rentConfig.maxTopUp, offset);
    offset += 2;

    return buffer;
}

/**
 * Serialize a light mint to buffer
 * Uses SPL's MintLayout for BaseMint, helper functions for context/metadata
 *
 * @param mint - The light mint to serialize
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

    // 2b. Encode mintSigner (32 bytes) and bump (1 byte)
    buffers.push(Buffer.from(mint.mintContext.mintSigner));
    buffers.push(Buffer.from([mint.mintContext.bump]));

    // 3. Encode reserved bytes (49 bytes) - default to zeros
    const reserved = mint.reserved ?? new Uint8Array(RESERVED_SIZE);
    buffers.push(Buffer.from(reserved));

    // 4. Encode account_type (1 byte) - default to ACCOUNT_TYPE_MINT (1)
    const accountType = mint.accountType ?? ACCOUNT_TYPE_MINT;
    buffers.push(Buffer.from([accountType]));

    // 5. Encode CompressionInfo (88 bytes) - default to zeros
    if (mint.compression) {
        buffers.push(serializeCompressionInfo(mint.compression));
    } else {
        buffers.push(Buffer.alloc(COMPRESSION_INFO_SIZE));
    }

    // 6. Encode extensions: Option<Vec<ExtensionStruct>>
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
    /** Signer pubkey used to derive the mint PDA */
    mintSigner: Uint8Array;
    /** Bump seed for the mint PDA */
    bump: number;
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
 * Convert a deserialized light mint to MintInstructionData format
 * This extracts and flattens the data structure for instruction encoding
 *
 * @param compressedMint - Deserialized light mint from account data
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
        mintSigner: mintContext.mintSigner,
        bump: mintContext.bump,
        metadata,
    };
}

/**
 * Convert a deserialized light mint to MintInstructionDataWithMetadata
 * Throws if the mint doesn't have metadata extension
 *
 * @param compressedMint - Deserialized light mint from account data
 * @returns MintInstructionDataWithMetadata for metadata update instructions
 * @throws Error if metadata extension is not present
 */
export function toMintInstructionDataWithMetadata(
    compressedMint: CompressedMint,
): MintInstructionDataWithMetadata {
    const data = toMintInstructionData(compressedMint);

    if (!data.metadata) {
        throw new Error('light mint does not have TokenMetadata extension');
    }

    return data as MintInstructionDataWithMetadata;
}
