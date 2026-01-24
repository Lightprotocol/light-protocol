/**
 * Create Associated Token Account actions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';

import { LIGHT_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID } from '../../constants.js';
import { deriveAssociatedTokenAddress } from '../../utils/derivation.js';
import {
    encodeCreateAtaInstructionData,
    defaultCompressibleParams,
} from '../../codecs/compressible.js';
import type { CompressibleExtensionInstructionData } from '../../codecs/types.js';

// ============================================================================
// CREATE ATA INSTRUCTION
// ============================================================================

/**
 * Parameters for creating an associated token account.
 */
export interface CreateAtaParams {
    /** Payer for the account creation */
    payer: Address;
    /** Owner of the token account */
    owner: Address;
    /** Mint address */
    mint: Address;
    /** Compressible config account (for rent-free accounts) */
    compressibleConfig: Address;
    /** Rent sponsor (for rent-free accounts) */
    rentSponsor: Address;
    /** Compressible extension params (optional, uses defaults) */
    compressibleParams?: CompressibleExtensionInstructionData;
    /** Whether to use idempotent variant (no-op if exists) */
    idempotent?: boolean;
}

/**
 * Result of ATA creation.
 */
export interface CreateAtaResult {
    /** The derived ATA address */
    address: Address;
    /** The PDA bump */
    bump: number;
    /** The instruction to create the ATA */
    instruction: IInstruction;
}

/**
 * Creates an associated token account instruction.
 *
 * @param params - ATA creation parameters
 * @returns The ATA address, bump, and instruction
 */
export async function createAssociatedTokenAccountInstruction(
    params: CreateAtaParams,
): Promise<CreateAtaResult> {
    const {
        payer,
        owner,
        mint,
        compressibleConfig,
        rentSponsor,
        compressibleParams = defaultCompressibleParams(),
        idempotent = false,
    } = params;

    // Derive the ATA address
    const { address: ata, bump } = await deriveAssociatedTokenAddress(
        owner,
        mint,
    );

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: owner, role: 0 }, // readonly
        { address: mint, role: 0 }, // readonly
        { address: payer, role: 3 }, // writable+signer
        { address: ata, role: 1 }, // writable
        { address: SYSTEM_PROGRAM_ID, role: 0 }, // readonly
        { address: compressibleConfig, role: 0 }, // readonly
        { address: rentSponsor, role: 1 }, // writable
    ];

    // Build instruction data
    const data = encodeCreateAtaInstructionData(
        {
            bump,
            compressibleConfig: compressibleParams,
        },
        idempotent,
    );

    const instruction: IInstruction = {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    return { address: ata, bump, instruction };
}

/**
 * Creates an idempotent ATA instruction (no-op if account exists).
 *
 * @param params - ATA creation parameters (idempotent flag ignored)
 * @returns The ATA address, bump, and instruction
 */
export async function createAssociatedTokenAccountIdempotentInstruction(
    params: Omit<CreateAtaParams, 'idempotent'>,
): Promise<CreateAtaResult> {
    return createAssociatedTokenAccountInstruction({
        ...params,
        idempotent: true,
    });
}
