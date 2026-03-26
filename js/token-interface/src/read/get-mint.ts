import { PublicKey, Commitment } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    Rpc,
    bn,
    deriveAddressV2,
    LIGHT_TOKEN_PROGRAM_ID,
    getDefaultAddressTreeInfo,
    MerkleContext,
    assertV2Enabled,
} from '@lightprotocol/stateless.js';
import {
    Mint,
    getMint as getSplMint,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    TokenAccountNotFoundError,
    TokenInvalidAccountOwnerError,
} from '@solana/spl-token';
import {
    deserializeMint,
    MintContext,
    TokenMetadata,
    MintExtension,
    extractTokenMetadata,
    CompressionInfo,
    CompressedMint,
} from '../instructions/layout/layout-mint';

export interface MintInfo {
    mint: Mint;
    programId: PublicKey;
    merkleContext?: MerkleContext;
    mintContext?: MintContext;
    tokenMetadata?: TokenMetadata;
    extensions?: MintExtension[];
    /** Compression info for light-token mints */
    compression?: CompressionInfo;
}

function toErrorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    return String(error);
}

/**
 * Get unified mint info for SPL/T22/light-token mints.
 *
 * @param rpc           RPC connection
 * @param address       The mint address
 * @param commitment    Optional commitment level
 * @param programId     Token program ID. If not provided, tries all programs to
 *                      auto-detect.
 * @returns Object with mint, optional merkleContext, mintContext, and
 * tokenMetadata
 */
export async function getMint(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<MintInfo> {
    assertV2Enabled();

    // try all three programs in parallel
    if (!programId) {
        const [tokenResult, token2022Result, compressedResult] =
            await Promise.allSettled([
                getMint(rpc, address, commitment, TOKEN_PROGRAM_ID),
                getMint(
                    rpc,
                    address,
                    commitment,
                    TOKEN_2022_PROGRAM_ID,
                ),
                getMint(
                    rpc,
                    address,
                    commitment,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
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

        const errors = [tokenResult, token2022Result, compressedResult]
            .filter(
                (result): result is PromiseRejectedResult =>
                    result.status === 'rejected',
            )
            .map(result => result.reason);

        const ownerMismatch = errors.find(
            error => error instanceof TokenInvalidAccountOwnerError,
        );
        if (ownerMismatch) {
            throw ownerMismatch;
        }

        const allNotFound =
            errors.length > 0 &&
            errors.every(error => error instanceof TokenAccountNotFoundError);
        if (allNotFound) {
            throw new TokenAccountNotFoundError(
                `Mint not found: ${address.toString()}. ` +
                    `Tried TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, and LIGHT_TOKEN_PROGRAM_ID.`,
            );
        }

        const unexpected = errors.find(
            error =>
                !(error instanceof TokenAccountNotFoundError) &&
                !(error instanceof TokenInvalidAccountOwnerError),
        );
        if (unexpected) {
            throw new Error(
                `Failed to fetch mint data from RPC: ${toErrorMessage(unexpected)}`,
            );
        }

        throw new TokenAccountNotFoundError(
            `Mint not found: ${address.toString()}. ` +
                `Tried TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, and LIGHT_TOKEN_PROGRAM_ID.`,
        );
    }

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        const addressTree = getDefaultAddressTreeInfo().tree;
        const compressedAddress = deriveAddressV2(
            address.toBytes(),
            addressTree,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        const compressedAccount = await rpc.getCompressedAccount(
            bn(compressedAddress.toBytes()),
        );

        if (!compressedAccount?.data?.data) {
            throw new TokenAccountNotFoundError(
                `Light mint not found for ${address.toString()}`,
            );
        }
        if (!compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
            throw new TokenInvalidAccountOwnerError();
        }

        const compressedData = Buffer.from(compressedAccount.data.data);

        // After decompressMint, the compressed account contains sentinel data (just hash ~32 bytes).
        // The actual mint data lives in the light mint account.
        // Minimum light mint size is 82 (base) + 34 (context) + 33 (signer+bump) = 149+ bytes.
        const SENTINEL_THRESHOLD = 64;
        const isDecompressed = compressedData.length < SENTINEL_THRESHOLD;

        let compressedMintData: CompressedMint;

        if (isDecompressed) {
            // Light mint account exists - read from light mint account
            const cmintAccountInfo = await rpc.getAccountInfo(
                address,
                commitment,
            );
            if (!cmintAccountInfo?.data) {
                throw new TokenAccountNotFoundError(
                    `Decompressed light mint account not found on-chain for ${address.toString()}`,
                );
            }
            if (!cmintAccountInfo.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
                throw new TokenInvalidAccountOwnerError();
            }
            compressedMintData = deserializeMint(
                Buffer.from(cmintAccountInfo.data),
            );
        } else {
            // Mint is still compressed - use compressed account data
            compressedMintData = deserializeMint(compressedData);
        }

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

        const result: MintInfo = {
            mint,
            programId,
            merkleContext,
            mintContext: compressedMintData.mintContext,
            tokenMetadata: tokenMetadata || undefined,
            extensions: compressedMintData.extensions || undefined,
            compression: compressedMintData.compression,
        };

        if (!result.merkleContext) {
            throw new Error(
                `Invalid light mint: merkleContext is required for LIGHT_TOKEN_PROGRAM_ID`,
            );
        }
        if (!result.mintContext) {
            throw new Error(
                `Invalid light mint: mintContext is required for LIGHT_TOKEN_PROGRAM_ID`,
            );
        }

        return result;
    }

    // Otherwise, fetch SPL/T22 mint
    const mint = await getSplMint(rpc, address, commitment, programId);
    return { mint, programId };
}
