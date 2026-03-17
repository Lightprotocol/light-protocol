/**
 * Create token account instruction.
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
import { encodeCreateTokenAccountInstructionData } from '../codecs/compressible.js';
import type { CompressibleExtensionInstructionData } from '../codecs/types.js';

/**
 * Parameters for creating a token account.
 */
export interface CreateTokenAccountParams {
    /** Token account address */
    tokenAccount: Address;
    /** Mint address */
    mint: Address;
    /** Owner of the token account */
    owner: Address;
    /** Compressible extension params (optional, enables compressible mode) */
    compressibleParams?: CompressibleExtensionInstructionData;
    /** Payer for account creation (required for compressible accounts) */
    payer?: Address;
    /** Compressible config account (defaults to LIGHT_TOKEN_CONFIG) */
    compressibleConfig?: Address;
    /** Rent sponsor PDA (defaults to LIGHT_TOKEN_RENT_SPONSOR) */
    rentSponsor?: Address;
    /** When true, emits SPL-compatible owner-only payload (non-compressible only) */
    splCompatibleOwnerOnlyData?: boolean;
}

/**
 * Creates a create token account instruction (discriminator: 18).
 *
 * Creates a CToken account for the given owner and mint.
 *
 * Account layout (non-compressible, owner-only data):
 * 0: token_account (writable) - SPL compatible, non-signer
 * 1: mint (readonly)
 *
 * Account layout (compressible):
 * 0: token_account (signer, writable) - created via CPI
 * 1: mint (readonly)
 * 2: payer (signer, writable)
 * 3: config_account (readonly) - CompressibleConfig
 * 4: system_program (readonly)
 * 5: rent_sponsor (writable)
 *
 * @param params - Create token account parameters
 * @returns The create token account instruction
 */
export function createTokenAccountInstruction(
    params: CreateTokenAccountParams,
): Instruction {
    const {
        tokenAccount,
        mint,
        owner,
        compressibleParams,
        payer,
        compressibleConfig,
        rentSponsor,
        splCompatibleOwnerOnlyData,
    } = params;

    const isCompressible = compressibleParams !== undefined;

    // Validate: payer/compressibleConfig/rentSponsor require compressibleParams
    if (!isCompressible && (payer !== undefined || compressibleConfig !== undefined || rentSponsor !== undefined)) {
        throw new Error('payer/compressibleConfig/rentSponsor require compressibleParams');
    }

    // Validate: splCompatibleOwnerOnlyData is only valid for non-compressible
    if (splCompatibleOwnerOnlyData && isCompressible) {
        throw new Error('splCompatibleOwnerOnlyData is only valid for non-compressible token account creation');
    }

    // Validate: compressibleParams requires payer
    if (isCompressible && !payer) {
        throw new Error('payer is required when compressibleParams is provided');
    }

    // Build accounts
    const accounts: AccountMeta[] = [
        {
            address: tokenAccount,
            role: isCompressible
                ? AccountRole.WRITABLE_SIGNER
                : AccountRole.WRITABLE,
        },
        { address: mint, role: AccountRole.READONLY },
    ];

    if (isCompressible) {
        accounts.push(
            { address: payer!, role: AccountRole.WRITABLE_SIGNER },
            { address: compressibleConfig ?? LIGHT_TOKEN_CONFIG, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: rentSponsor ?? LIGHT_TOKEN_RENT_SPONSOR, role: AccountRole.WRITABLE },
        );
    }

    // Build instruction data
    const useSplOwnerOnly = splCompatibleOwnerOnlyData === true;
    const data = encodeCreateTokenAccountInstructionData(
        {
            owner,
            compressibleConfig: compressibleParams ?? null,
        },
        useSplOwnerOnly,
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
