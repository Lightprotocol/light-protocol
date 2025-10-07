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
    /** Whether the associated SPL mint is initialized */
    splMintInitialized: boolean;
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
 * Parsed token metadata (name, symbol, uri, etc.)
 */
export interface TokenMetadata {
    name: string;
    symbol: string;
    uri: string;
    updateAuthority?: PublicKey | null;
    additionalMetadata?: { key: string; value: string }[];
}

/**
 * Borsh layout for TokenMetadata
 * Matches the on-chain format exactly
 */
export const TokenMetadataLayout = borshStruct([
    option(borshPublicKey(), 'updateAuthority'),
    vecU8('name'),
    vecU8('symbol'),
    vecU8('uri'),
    option(
        vec(borshStruct([vecU8('key'), vecU8('value')]), 'items'),
        'additionalMetadata',
    ),
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
    splMintInitialized: number; // bool as u8
    splMint: PublicKey;
}

/** Buffer layout for de/serializing MintContext */
export const MintContextLayout = struct<RawMintContext>([
    u8('version'),
    u8('splMintInitialized'),
    publicKey('splMint'),
]);

/** Byte length of MintContext */
export const MINT_CONTEXT_SIZE = MintContextLayout.span; // 34 bytes

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

    // 3. Parse extensions: Option<Vec<MintExtension>>
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

            const dataLen = buffer.readUInt32LE(offset);
            offset += 4;
            const extensionData = buffer.slice(offset, offset + dataLen);
            offset += dataLen;

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
        splMintInitialized: rawContext.splMintInitialized !== 0,
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
            splMintInitialized: mint.mintContext.splMintInitialized ? 1 : 0,
            splMint: mint.mintContext.splMint,
        },
        contextBuffer,
    );
    buffers.push(contextBuffer);

    // 3. Encode extensions: Option<Vec<MintExtension>>
    if (mint.extensions && mint.extensions.length > 0) {
        buffers.push(Buffer.from([1])); // Some
        const vecLenBuf = Buffer.alloc(4);
        vecLenBuf.writeUInt32LE(mint.extensions.length);
        buffers.push(vecLenBuf);

        for (const ext of mint.extensions) {
            buffers.push(Buffer.from([ext.extensionType]));
            const dataLenBuf = Buffer.alloc(4);
            dataLenBuf.writeUInt32LE(ext.data.length);
            buffers.push(dataLenBuf);
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
 * @param data - Raw extension data
 * @returns Parsed TokenMetadata or null if invalid
 */
export function decodeTokenMetadata(data: Uint8Array): TokenMetadata | null {
    try {
        const buffer = Buffer.from(data);
        if (buffer.length < 1) {
            return null;
        }

        const decoded = TokenMetadataLayout.decode(buffer, 0) as any;

        // Convert Vec<u8> buffers to strings
        const name = Buffer.from(decoded.name).toString('utf-8');
        const symbol = Buffer.from(decoded.symbol).toString('utf-8');
        const uri = Buffer.from(decoded.uri).toString('utf-8');

        // Convert additionalMetadata if present
        let additionalMetadata: { key: string; value: string }[] | undefined;
        if (decoded.additionalMetadata && decoded.additionalMetadata.items) {
            additionalMetadata = decoded.additionalMetadata.items.map(
                (item: any) => ({
                    key: Buffer.from(item.key).toString('utf-8'),
                    value: Buffer.from(item.value).toString('utf-8'),
                }),
            );
        }

        return {
            name,
            symbol,
            uri,
            updateAuthority: decoded.updateAuthority || undefined,
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
    const len = TokenMetadataLayout.encode(
        {
            updateAuthority: metadata.updateAuthority || null,
            name: Buffer.from(metadata.name),
            symbol: Buffer.from(metadata.symbol),
            uri: Buffer.from(metadata.uri),
            additionalMetadata: metadata.additionalMetadata
                ? {
                      items: metadata.additionalMetadata.map(item => ({
                          key: Buffer.from(item.key),
                          value: Buffer.from(item.value),
                      })),
                  }
                : null,
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
