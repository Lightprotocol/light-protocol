import { PublicKey, AccountInfo, Commitment } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    Rpc,
    bn,
    deriveAddressV2,
    CTOKEN_PROGRAM_ID,
    getDefaultAddressTreeInfo,
    MerkleContext,
    assertBetaEnabled,
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
    MintContext,
    TokenMetadata,
    MintExtension,
    extractTokenMetadata,
    CompressionInfo,
    CompressedMint,
} from './layout/layout-mint';

export interface MintInterface {
    mint: Mint;
    programId: PublicKey;
    merkleContext?: MerkleContext;
    mintContext?: MintContext;
    tokenMetadata?: TokenMetadata;
    extensions?: MintExtension[];
    /** Compression info for c-token mints */
    compression?: CompressionInfo;
}

/**
 * Get unified mint info for SPL/T22/c-token mints.
 *
 * @param rpc           RPC connection
 * @param address       The mint address
 * @param commitment    Optional commitment level
 * @param programId     Token program ID. If not provided, tries all programs to
 *                      auto-detect.
 * @returns Object with mint, optional merkleContext, mintContext, and
 * tokenMetadata
 */
export async function getMintInterface(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<MintInterface> {
    assertBetaEnabled();

    // try all three programs in parallel
    if (!programId) {
        const [tokenResult, token2022Result, compressedResult] =
            await Promise.allSettled([
                getMintInterface(rpc, address, commitment, TOKEN_PROGRAM_ID),
                getMintInterface(
                    rpc,
                    address,
                    commitment,
                    TOKEN_2022_PROGRAM_ID,
                ),
                getMintInterface(rpc, address, commitment, CTOKEN_PROGRAM_ID),
            ]);

        if (tokenResult.status === 'fulfilled') {
            return tokenResult.value;
        }
        if (token2022Result.status === 'fulfilled') {
            return token2022Result.value;
        }
        if (compressedResult.status === 'fulfilled') {
            return compressedResult.value;
        }

        throw new Error(
            `Mint not found: ${address.toString()}. ` +
                `Tried TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, and CTOKEN_PROGRAM_ID.`,
        );
    }

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        const addressTree = getDefaultAddressTreeInfo().tree;
        const compressedAddress = deriveAddressV2(
            address.toBytes(),
            addressTree,
            CTOKEN_PROGRAM_ID,
        );
        const compressedAccount = await rpc.getCompressedAccount(
            bn(compressedAddress.toBytes()),
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

        const tokenMetadata = extractTokenMetadata(
            compressedMintData.extensions,
        );

        const result: MintInterface = {
            mint,
            programId,
            merkleContext,
            mintContext: compressedMintData.mintContext,
            tokenMetadata: tokenMetadata || undefined,
            extensions: compressedMintData.extensions || undefined,
            compression: compressedMintData.compression,
        };

        if (programId.equals(CTOKEN_PROGRAM_ID)) {
            if (!result.merkleContext) {
                throw new Error(
                    `Invalid compressed mint: merkleContext is required for CTOKEN_PROGRAM_ID`,
                );
            }
            if (!result.mintContext) {
                throw new Error(
                    `Invalid compressed mint: mintContext is required for CTOKEN_PROGRAM_ID`,
                );
            }
        }

        return result;
    }

    // Otherwise, fetch SPL/T22 mint
    const mint = await getSplMint(rpc, address, commitment, programId);
    return { mint, programId };
}

/**
 * Unpack mint info from raw account data for SPL/T22/c-token.
 *
 * @param address       The mint pubkey
 * @param data          The raw account data or AccountInfo
 * @param programId     Token program ID. Default c-token.
 * @returns Object with mint, optional mintContext and tokenMetadata.
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

        const result = {
            mint,
            programId,
            mintContext: compressedMintData.mintContext,
            tokenMetadata: tokenMetadata || undefined,
            extensions: compressedMintData.extensions || undefined,
            compression: compressedMintData.compression,
        };

        // Validate: CTOKEN_PROGRAM_ID requires mintContext
        if (programId.equals(CTOKEN_PROGRAM_ID)) {
            if (!result.mintContext) {
                throw new Error(
                    `Invalid compressed mint: mintContext is required for CTOKEN_PROGRAM_ID`,
                );
            }
        }

        return result;
    }

    // Otherwise, unpack as SPL mint
    const info = data as AccountInfo<Buffer>;
    const mint = unpackSplMint(address, info, programId);
    return { mint, programId };
}

/**
 * Unpack c-token mint context and metadata from raw account data
 *
 * @param data  The raw account data
 * @returns     Object with mintContext, tokenMetadata, and extensions
 */
export function unpackMintData(data: Buffer | Uint8Array): {
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
