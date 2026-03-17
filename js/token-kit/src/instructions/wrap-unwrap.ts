/**
 * Wrap (SPL → Light Token) and Unwrap (Light Token → SPL) instruction builders.
 *
 * Both use Transfer2 Path A (compression-only) with two compression structs.
 */

import type { Address } from '@solana/addresses';
import { AccountRole, type Instruction } from '@solana/instructions';

import type { SplInterfaceInfo } from '../utils/spl-interface.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    CPI_AUTHORITY,
    SYSTEM_PROGRAM_ID,
} from '../constants.js';
import { encodeTransfer2InstructionData } from '../codecs/transfer2.js';
import {
    createCompressSpl,
    createDecompress,
    createCompress,
    createDecompressSpl,
} from './transfer2.js';

// Packed account indices (relative to the packed accounts array after Path A prefix)
const MINT_INDEX = 0;
const OWNER_INDEX = 1;
const SOURCE_INDEX = 2;
const DESTINATION_INDEX = 3;
const POOL_INDEX = 4;
// SPL_TOKEN_PROGRAM_INDEX = 5 (unused in compression structs but present in accounts)
const CTOKEN_PROGRAM_INDEX = 6;

// ============================================================================
// WRAP INSTRUCTION
// ============================================================================

/**
 * Parameters for creating a wrap instruction (SPL → Light Token).
 */
export interface WrapParams {
    /** Source SPL token account (writable) */
    source: Address;
    /** Destination Light Token account (writable) */
    destination: Address;
    /** Owner of the source account (signer) */
    owner: Address;
    /** Token mint address */
    mint: Address;
    /** Amount to wrap */
    amount: bigint;
    /** SPL interface pool info */
    splInterfaceInfo: SplInterfaceInfo;
    /** Mint decimals */
    decimals: number;
    /** Fee payer (defaults to owner) */
    feePayer?: Address;
}

/**
 * Creates a wrap instruction that moves tokens from an SPL/Token 2022 account
 * to a Light Token account.
 *
 * Uses Transfer2 Path A (compression-only) with two compressions:
 *   1. compressSpl: burns from SPL associated token account into the pool
 *   2. decompressCtoken: mints from pool into Light Token associated token account
 *
 * Account layout:
 *   0: CPI_AUTHORITY (readonly)
 *   1: feePayer (writable signer)
 *   2: mint (readonly)              — packed index 0
 *   3: owner (signer)               — packed index 1
 *   4: source (writable)            — packed index 2
 *   5: destination (writable)       — packed index 3
 *   6: poolPda (writable)           — packed index 4
 *   7: tokenProgram (readonly)      — packed index 5
 *   8: LIGHT_TOKEN_PROGRAM_ID       — packed index 6
 *   9: SYSTEM_PROGRAM_ID            — packed index 7
 */
export function createWrapInstruction(params: WrapParams): Instruction {
    const {
        source,
        destination,
        owner,
        mint,
        amount,
        splInterfaceInfo,
        decimals,
        feePayer,
    } = params;

    const payer = feePayer ?? owner;

    const compressions = [
        createCompressSpl({
            amount,
            mintIndex: MINT_INDEX,
            sourceIndex: SOURCE_INDEX,
            authorityIndex: OWNER_INDEX,
            poolAccountIndex: POOL_INDEX,
            poolIndex: splInterfaceInfo.poolIndex,
            bump: splInterfaceInfo.bump,
            decimals,
        }),
        createDecompress({
            amount,
            mintIndex: MINT_INDEX,
            recipientIndex: DESTINATION_INDEX,
            tokenProgramIndex: CTOKEN_PROGRAM_INDEX,
        }),
    ];

    const data = encodeTransfer2InstructionData({
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0,
        maxTopUp: 0,
        cpiContext: null,
        compressions,
        proof: null,
        inTokenData: [],
        outTokenData: [],
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    });

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts: [
            // Path A prefix
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: payer, role: AccountRole.WRITABLE_SIGNER },
            // Packed accounts
            { address: mint, role: AccountRole.READONLY },
            { address: owner, role: AccountRole.READONLY_SIGNER },
            { address: source, role: AccountRole.WRITABLE },
            { address: destination, role: AccountRole.WRITABLE },
            {
                address: splInterfaceInfo.poolAddress,
                role: AccountRole.WRITABLE,
            },
            {
                address: splInterfaceInfo.tokenProgram,
                role: AccountRole.READONLY,
            },
            { address: LIGHT_TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        ],
        data,
    };
}

// ============================================================================
// UNWRAP INSTRUCTION
// ============================================================================

/**
 * Parameters for creating an unwrap instruction (Light Token → SPL).
 */
export interface UnwrapParams {
    /** Source Light Token account (writable) */
    source: Address;
    /** Destination SPL token account (writable) */
    destination: Address;
    /** Owner of the source account (signer) */
    owner: Address;
    /** Token mint address */
    mint: Address;
    /** Amount to unwrap */
    amount: bigint;
    /** SPL interface pool info */
    splInterfaceInfo: SplInterfaceInfo;
    /** Mint decimals */
    decimals: number;
    /** Fee payer (defaults to owner) */
    feePayer?: Address;
}

/**
 * Creates an unwrap instruction that moves tokens from a Light Token account
 * to an SPL/Token 2022 account.
 *
 * Uses Transfer2 Path A (compression-only) with two compressions:
 *   1. compressCtoken: burns from Light Token associated token account into the pool
 *   2. decompressSpl: mints from pool into SPL associated token account
 *
 * Account layout matches wrap for consistency.
 */
export function createUnwrapInstruction(params: UnwrapParams): Instruction {
    const {
        source,
        destination,
        owner,
        mint,
        amount,
        splInterfaceInfo,
        decimals,
        feePayer,
    } = params;

    const payer = feePayer ?? owner;

    const compressions = [
        createCompress({
            amount,
            mintIndex: MINT_INDEX,
            sourceIndex: SOURCE_INDEX,
            authorityIndex: OWNER_INDEX,
            tokenProgramIndex: CTOKEN_PROGRAM_INDEX,
        }),
        createDecompressSpl({
            amount,
            mintIndex: MINT_INDEX,
            recipientIndex: DESTINATION_INDEX,
            poolAccountIndex: POOL_INDEX,
            poolIndex: splInterfaceInfo.poolIndex,
            bump: splInterfaceInfo.bump,
            decimals,
        }),
    ];

    const data = encodeTransfer2InstructionData({
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0,
        maxTopUp: 0,
        cpiContext: null,
        compressions,
        proof: null,
        inTokenData: [],
        outTokenData: [],
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    });

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts: [
            // Path A prefix
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: payer, role: AccountRole.WRITABLE_SIGNER },
            // Packed accounts
            { address: mint, role: AccountRole.READONLY },
            { address: owner, role: AccountRole.READONLY_SIGNER },
            { address: source, role: AccountRole.WRITABLE },
            { address: destination, role: AccountRole.WRITABLE },
            {
                address: splInterfaceInfo.poolAddress,
                role: AccountRole.WRITABLE,
            },
            {
                address: splInterfaceInfo.tokenProgram,
                role: AccountRole.READONLY,
            },
            { address: LIGHT_TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        ],
        data,
    };
}
