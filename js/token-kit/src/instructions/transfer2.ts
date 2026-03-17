/**
 * Transfer2 instruction builder and compression factory helpers.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID,
    CPI_AUTHORITY,
    REGISTERED_PROGRAM_PDA,
    ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    ACCOUNT_COMPRESSION_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    COMPRESSION_MODE,
} from '../constants.js';
import { encodeTransfer2InstructionData } from '../codecs/transfer2.js';
import type {
    Compression,
    Transfer2InstructionData,
} from '../codecs/types.js';

// ============================================================================
// TRANSFER2 INSTRUCTION
// ============================================================================

/**
 * Parameters for Transfer2 instruction.
 */
export interface Transfer2Params {
    /** Fee payer (signer, writable) */
    feePayer: Address;
    /** Full Transfer2 instruction data */
    data: Transfer2InstructionData;
    /** SOL pool PDA (optional, writable) */
    solPoolPda?: Address;
    /** SOL decompression recipient (optional, writable) */
    solDecompressionRecipient?: Address;
    /** CPI context account (optional, writable) — triggers Path C */
    cpiContextAccount?: Address;
    /** Packed remaining accounts (mints, owners, delegates, trees, queues) */
    packedAccounts: AccountMeta[];
}

/**
 * Creates a Transfer2 instruction (discriminator: 101).
 *
 * Transfer2 supports batch transfers between compressed and decompressed
 * token accounts, including compress and decompress operations.
 *
 * Path A (compression-only): compressions set, no inTokenData/outTokenData
 *   0: cpiAuthorityPda (readonly)
 *   1: feePayer (writable signer)
 *   [...packed_accounts]
 *
 * Path B (full transfer): inTokenData or outTokenData present, no cpiContextAccount
 *   0: lightSystemProgram (readonly)
 *   1: feePayer (writable signer)
 *   2: cpiAuthorityPda (readonly)
 *   3: registeredProgramPda (readonly)
 *   4: accountCompressionAuthority (readonly)
 *   5: accountCompressionProgram (readonly)
 *   6: systemProgram (readonly)
 *   [...packed_accounts]
 *
 * Path C (CPI context write): cpiContextAccount provided
 *   0: lightSystemProgram (readonly)
 *   1: feePayer (writable signer)
 *   2: cpiAuthorityPda (readonly)
 *   3: cpiContextAccount (writable)
 *   [...packed_accounts]
 *
 * @param params - Transfer2 parameters
 * @returns The Transfer2 instruction
 */
export function createTransfer2Instruction(
    params: Transfer2Params,
): Instruction {
    const {
        feePayer,
        data: transferData,
        solPoolPda,
        solDecompressionRecipient,
        cpiContextAccount,
        packedAccounts,
    } = params;

    const hasInOrOut =
        (transferData.inTokenData && transferData.inTokenData.length > 0) ||
        (transferData.outTokenData && transferData.outTokenData.length > 0);

    const accounts: AccountMeta[] = [];

    if (cpiContextAccount) {
        // Path C: CPI context write
        accounts.push(
            { address: LIGHT_SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: feePayer, role: AccountRole.WRITABLE_SIGNER },
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: cpiContextAccount, role: AccountRole.WRITABLE },
        );
    } else if (hasInOrOut) {
        // Path B: full transfer with Light system accounts
        accounts.push(
            { address: LIGHT_SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: feePayer, role: AccountRole.WRITABLE_SIGNER },
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: REGISTERED_PROGRAM_PDA, role: AccountRole.READONLY },
            {
                address: ACCOUNT_COMPRESSION_AUTHORITY_PDA,
                role: AccountRole.READONLY,
            },
            {
                address: ACCOUNT_COMPRESSION_PROGRAM_ID,
                role: AccountRole.READONLY,
            },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        );
    } else {
        // Path A: compression-only (no system CPI needed)
        accounts.push(
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: feePayer, role: AccountRole.WRITABLE_SIGNER },
        );
    }

    // Add optional accounts (only for Path B)
    if (!cpiContextAccount && hasInOrOut) {
        if (solPoolPda) {
            accounts.push({ address: solPoolPda, role: AccountRole.WRITABLE });
        }
        if (solDecompressionRecipient) {
            accounts.push({
                address: solDecompressionRecipient,
                role: AccountRole.WRITABLE,
            });
        }
    }

    // Add packed remaining accounts
    accounts.push(...packedAccounts);

    // Encode instruction data
    const encodedData = encodeTransfer2InstructionData(transferData);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data: encodedData,
    };
}

// ============================================================================
// COMPRESSION FACTORY HELPERS
// ============================================================================

/**
 * Parameters for creating a CToken compression struct.
 */
interface CompressParams {
    amount: bigint;
    mintIndex: number;
    sourceIndex: number;
    authorityIndex: number;
}

/**
 * Parameters for creating an SPL compression struct.
 */
interface CompressSplParams extends CompressParams {
    poolAccountIndex: number;
    poolIndex: number;
    bump: number;
    decimals: number;
}

/**
 * Parameters for creating a CToken decompression struct.
 */
interface DecompressParams {
    amount: bigint;
    mintIndex: number;
    recipientIndex: number;
}

/**
 * Parameters for creating an SPL decompression struct.
 */
interface DecompressSplParams extends DecompressParams {
    poolAccountIndex: number;
    poolIndex: number;
    bump: number;
    decimals: number;
}

/**
 * Parameters for creating a compress-and-close struct.
 */
interface CompressAndCloseParams {
    amount: bigint;
    mintIndex: number;
    sourceIndex: number;
    authorityIndex: number;
    rentSponsorIndex: number;
    compressedAccountIndex: number;
    destinationIndex: number;
}

/**
 * Creates a Compression struct for compressing CTokens.
 */
export function createCompress(params: CompressParams): Compression {
    return {
        mode: COMPRESSION_MODE.COMPRESS,
        amount: params.amount,
        mint: params.mintIndex,
        sourceOrRecipient: params.sourceIndex,
        authority: params.authorityIndex,
        poolAccountIndex: 0,
        poolIndex: 0,
        bump: 0,
        decimals: 0,
    };
}

/**
 * Creates a Compression struct for compressing SPL tokens.
 */
export function createCompressSpl(params: CompressSplParams): Compression {
    return {
        mode: COMPRESSION_MODE.COMPRESS,
        amount: params.amount,
        mint: params.mintIndex,
        sourceOrRecipient: params.sourceIndex,
        authority: params.authorityIndex,
        poolAccountIndex: params.poolAccountIndex,
        poolIndex: params.poolIndex,
        bump: params.bump,
        decimals: params.decimals,
    };
}

/**
 * Creates a Compression struct for decompressing CTokens.
 */
export function createDecompress(params: DecompressParams): Compression {
    return {
        mode: COMPRESSION_MODE.DECOMPRESS,
        amount: params.amount,
        mint: params.mintIndex,
        sourceOrRecipient: params.recipientIndex,
        authority: 0,
        poolAccountIndex: 0,
        poolIndex: 0,
        bump: 0,
        decimals: 0,
    };
}

/**
 * Creates a Compression struct for decompressing SPL tokens.
 */
export function createDecompressSpl(
    params: DecompressSplParams,
): Compression {
    return {
        mode: COMPRESSION_MODE.DECOMPRESS,
        amount: params.amount,
        mint: params.mintIndex,
        sourceOrRecipient: params.recipientIndex,
        authority: 0,
        poolAccountIndex: params.poolAccountIndex,
        poolIndex: params.poolIndex,
        bump: params.bump,
        decimals: params.decimals,
    };
}

/**
 * Creates a Compression struct for compressing and closing an account.
 *
 * Repurposed fields:
 * - poolAccountIndex = rentSponsorIndex
 * - poolIndex = compressedAccountIndex
 * - bump = destinationIndex
 */
export function createCompressAndClose(
    params: CompressAndCloseParams,
): Compression {
    return {
        mode: COMPRESSION_MODE.COMPRESS_AND_CLOSE,
        amount: params.amount,
        mint: params.mintIndex,
        sourceOrRecipient: params.sourceIndex,
        authority: params.authorityIndex,
        poolAccountIndex: params.rentSponsorIndex,
        poolIndex: params.compressedAccountIndex,
        bump: params.destinationIndex,
        decimals: 0,
    };
}
