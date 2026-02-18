/**
 * Create Associated Token Account instruction.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
} from '../constants.js';
import { deriveAssociatedTokenAddress } from '../utils/derivation.js';
import {
    encodeCreateAtaInstructionData,
    defaultCompressibleParams,
} from '../codecs/compressible.js';
import type { CompressibleExtensionInstructionData } from '../codecs/types.js';

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
    /** Compressible config account (defaults to LIGHT_TOKEN_CONFIG) */
    compressibleConfig?: Address;
    /** Rent sponsor PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR) */
    rentSponsor?: Address;
    /** Compressible extension params (optional, uses production defaults) */
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
    instruction: Instruction;
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
        compressibleConfig = LIGHT_TOKEN_CONFIG,
        rentSponsor = LIGHT_TOKEN_RENT_SPONSOR,
        compressibleParams = defaultCompressibleParams(),
        idempotent = false,
    } = params;

    // Derive the ATA address
    const { address: ata, bump } = await deriveAssociatedTokenAddress(
        owner,
        mint,
    );

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: owner, role: AccountRole.READONLY },
        { address: mint, role: AccountRole.READONLY },
        { address: payer, role: AccountRole.WRITABLE_SIGNER },
        { address: ata, role: AccountRole.WRITABLE },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        { address: compressibleConfig, role: AccountRole.READONLY },
        { address: rentSponsor, role: AccountRole.WRITABLE },
    ];

    // Build instruction data
    const data = encodeCreateAtaInstructionData(
        {
            compressibleConfig: compressibleParams,
        },
        idempotent,
    );

    const instruction: Instruction = {
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
