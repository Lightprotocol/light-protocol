/**
 * SPL interface PDA instruction builders.
 *
 * Creates and manages SPL interface PDAs that register mints with
 * the Light Token Program, enabling compress/decompress operations.
 */

import type { Address } from '@solana/addresses';
import { AccountRole, type Instruction } from '@solana/instructions';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID,
} from '../constants.js';
import { derivePoolAddress } from '../utils/derivation.js';

// ============================================================================
// CREATE SPL INTERFACE
// ============================================================================

/**
 * Parameters for creating an SPL interface PDA instruction.
 */
export interface CreateSplInterfaceParams {
    /** Fee payer (signer, writable) */
    feePayer: Address;
    /** Token mint address */
    mint: Address;
    /** Token program (SPL Token or Token 2022) */
    tokenProgram?: Address;
}

/**
 * Result of creating an SPL interface instruction.
 */
export interface CreateSplInterfaceResult {
    /** The instruction to create the SPL interface PDA */
    instruction: Instruction;
    /** The derived pool PDA address */
    poolAddress: Address;
    /** The PDA bump */
    bump: number;
}

/**
 * Creates an instruction to register an SPL interface PDA for a mint.
 *
 * This registers the mint with the Light Token Program, enabling
 * compress and decompress operations for the mint's tokens.
 *
 * Account layout (matches CompressedTokenProgram.createTokenPool):
 *   0: feePayer (writable signer)
 *   1: tokenPoolPda (writable)
 *   2: systemProgram (readonly)
 *   3: mint (readonly)
 *   4: tokenProgram (readonly)
 *   5: cTokenProgram (readonly)
 *
 * @param params - Create SPL interface parameters
 * @returns The instruction and derived pool info
 */
export async function createSplInterfaceInstruction(
    params: CreateSplInterfaceParams,
): Promise<CreateSplInterfaceResult> {
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const { address: poolAddress, bump } = await derivePoolAddress(
        params.mint,
        0,
    );

    // Discriminator for create_token_pool (8-byte Anchor discriminator)
    // Matches CompressedTokenProgram.createTokenPool
    const discriminator = new Uint8Array([
        0x3c, 0xb4, 0x0e, 0x78, 0x03, 0x0a, 0xd3, 0x04,
    ]);

    const instruction: Instruction = {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts: [
            {
                address: params.feePayer,
                role: AccountRole.WRITABLE_SIGNER,
            },
            { address: poolAddress, role: AccountRole.WRITABLE },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: params.mint, role: AccountRole.READONLY },
            { address: tokenProgram, role: AccountRole.READONLY },
            {
                address: LIGHT_TOKEN_PROGRAM_ID,
                role: AccountRole.READONLY,
            },
        ],
        data: discriminator,
    };

    return { instruction, poolAddress, bump };
}

// ============================================================================
// ADD SPL INTERFACES (MULTI-POOL)
// ============================================================================

/**
 * Parameters for adding additional SPL interface PDAs.
 */
export interface AddSplInterfacesParams {
    /** Fee payer (signer, writable) */
    feePayer: Address;
    /** Token mint address */
    mint: Address;
    /** Token program (SPL Token or Token 2022) */
    tokenProgram?: Address;
    /** Number of additional pools to create (up to 4 total, indices 1-4) */
    count?: number;
    /** Existing pool indices to skip (already initialized) */
    existingIndices?: number[];
}

/**
 * Creates instructions to add additional SPL interface PDAs for a mint.
 *
 * Mints can have up to 5 pool PDAs (indices 0-4). Index 0 is created by
 * createSplInterfaceInstruction. This function creates PDAs for indices
 * 1 through count, skipping any already-initialized indices.
 *
 * @param params - Add SPL interfaces parameters
 * @returns Array of instructions, one per new pool PDA
 */
export async function addSplInterfacesInstruction(
    params: AddSplInterfacesParams,
): Promise<CreateSplInterfaceResult[]> {
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const count = params.count ?? 4;
    const existingSet = new Set(params.existingIndices ?? [0]);

    // Discriminator for add_token_pool (8-byte Anchor discriminator)
    const discriminator = new Uint8Array([
        0xf2, 0x39, 0xc1, 0x2b, 0x97, 0x96, 0xbe, 0x55,
    ]);

    const results: CreateSplInterfaceResult[] = [];

    for (let i = 1; i <= count; i++) {
        if (existingSet.has(i)) continue;

        const { address: poolAddress, bump } = await derivePoolAddress(
            params.mint,
            i,
        );

        const instruction: Instruction = {
            programAddress: LIGHT_TOKEN_PROGRAM_ID,
            accounts: [
                {
                    address: params.feePayer,
                    role: AccountRole.WRITABLE_SIGNER,
                },
                { address: poolAddress, role: AccountRole.WRITABLE },
                {
                    address: SYSTEM_PROGRAM_ID,
                    role: AccountRole.READONLY,
                },
                { address: params.mint, role: AccountRole.READONLY },
                { address: tokenProgram, role: AccountRole.READONLY },
                {
                    address: LIGHT_TOKEN_PROGRAM_ID,
                    role: AccountRole.READONLY,
                },
            ],
            data: discriminator,
        };

        results.push({ instruction, poolAddress, bump });
    }

    return results;
}
