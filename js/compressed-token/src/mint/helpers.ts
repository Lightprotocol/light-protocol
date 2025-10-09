import { PublicKey, AccountInfo, Commitment } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    Rpc,
    bn,
    deriveAddressV2,
    CTOKEN_PROGRAM_ID,
    getDefaultAddressTreeInfo,
    MerkleContext,
} from '@lightprotocol/stateless.js';
import {
    Mint,
    getMint as getSplMint,
    unpackMint as unpackSplMint,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import {
    deserializeMint,
    CompressedMint,
    MintContext,
    TokenMetadata,
    MintExtension,
    extractTokenMetadata,
} from './serde';

export interface MintInterface {
    mint: Mint;
    merkleContext?: MerkleContext;
    mintContext?: MintContext;
    tokenMetadata?: TokenMetadata; // Parsed metadata (first-class)
    extensions?: MintExtension[]; // Raw extensions array (optional)
}

/**
 * Get mint interface - supports both SPL and compressed mints
 * Supports TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID (SPL), and CTOKEN_PROGRAM_ID (compressed)
 *
 * @param rpc - RPC connection
 * @param address - The mint address
 * @param commitment - Optional commitment level
 * @param programId - Token program ID (defaults to TOKEN_PROGRAM_ID)
 * @returns Object with mint, optional merkleContext, mintContext, and tokenMetadata for compressed mints
 */
export async function getMintInterface(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId: PublicKey = TOKEN_PROGRAM_ID,
): Promise<MintInterface> {
    // If programId is compressed token program, fetch compressed mint
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        const addressTree = getDefaultAddressTreeInfo().tree;
        const compressedAddress = deriveAddressV2(
            address.toBytes(),
            addressTree.toBytes(),
            CTOKEN_PROGRAM_ID.toBytes(),
        );
        const compressedAccount = await rpc.getCompressedAccount(
            bn(Array.from(compressedAddress)),
        );

        if (!compressedAccount?.data?.data) {
            throw new Error(
                `Compressed mint not found for ${address.toString()}`,
            );
        }

        const compressedMintData = deserializeMint(
            Buffer.from(compressedAccount.data.data),
        );

        const mint: Mint = {
            address,
            mintAuthority: compressedMintData.base.mintAuthority,
            supply: compressedMintData.base.supply,
            decimals: compressedMintData.base.decimals,
            isInitialized: compressedMintData.base.isInitialized,
            freezeAuthority: compressedMintData.base.freezeAuthority,
            tlvData: Buffer.alloc(0),
        };

        const merkleContext: MerkleContext = {
            treeInfo: compressedAccount.treeInfo,
            hash: compressedAccount.hash,
            leafIndex: compressedAccount.leafIndex,
            proveByIndex: compressedAccount.proveByIndex,
        };

        // Extract and parse TokenMetadata
        const tokenMetadata = extractTokenMetadata(
            compressedMintData.extensions,
        );

        return {
            mint,
            merkleContext,
            mintContext: compressedMintData.mintContext,
            tokenMetadata: tokenMetadata || undefined,
            extensions: compressedMintData.extensions || undefined,
        };
    }

    // Otherwise, fetch SPL mint (TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID)
    const mint = await getSplMint(rpc, address, commitment, programId);
    return { mint };
}

/**
 * Unpack mint interface from raw account data
 * Handles both SPL and compressed mint formats
 * Note: merkleContext not available from raw data, use getMintInterface for full context
 *
 * @param address - The mint pubkey
 * @param data - The raw account data or AccountInfo
 * @param programId - Token program ID (defaults to TOKEN_PROGRAM_ID)
 * @returns Object with mint, optional mintContext and tokenMetadata for compressed mints
 */
export function unpackMintInterface(
    address: PublicKey,
    data: Buffer | Uint8Array | AccountInfo<Buffer>,
    programId: PublicKey = TOKEN_PROGRAM_ID,
): Omit<MintInterface, 'merkleContext'> {
    const buffer =
        data instanceof Buffer
            ? data
            : data instanceof Uint8Array
              ? Buffer.from(data)
              : data.data;

    // If compressed token program, deserialize as compressed mint
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        const compressedMintData = deserializeMint(buffer);

        const mint: Mint = {
            address,
            mintAuthority: compressedMintData.base.mintAuthority,
            supply: compressedMintData.base.supply,
            decimals: compressedMintData.base.decimals,
            isInitialized: compressedMintData.base.isInitialized,
            freezeAuthority: compressedMintData.base.freezeAuthority,
            tlvData: Buffer.alloc(0),
        };

        // Extract and parse TokenMetadata
        const tokenMetadata = extractTokenMetadata(
            compressedMintData.extensions,
        );

        return {
            mint,
            mintContext: compressedMintData.mintContext,
            tokenMetadata: tokenMetadata || undefined,
            extensions: compressedMintData.extensions || undefined,
        };
    }

    // Otherwise, unpack as SPL mint
    const info = data as AccountInfo<Buffer>;
    const mint = unpackSplMint(address, info, programId);
    return { mint };
}

/**
 * Unpack compressed mint context and metadata from raw account data
 *
 * @param data - The raw account data
 * @returns Object with mintContext, tokenMetadata, and extensions
 */
export function unpackCompressedMintData(data: Buffer | Uint8Array): {
    mintContext: MintContext;
    tokenMetadata?: TokenMetadata;
    extensions?: MintExtension[];
} {
    const buffer = data instanceof Buffer ? data : Buffer.from(data);
    const compressedMint = deserializeMint(buffer);
    const tokenMetadata = extractTokenMetadata(compressedMint.extensions);

    return {
        mintContext: compressedMint.mintContext,
        tokenMetadata: tokenMetadata || undefined,
        extensions: compressedMint.extensions || undefined,
    };
}
