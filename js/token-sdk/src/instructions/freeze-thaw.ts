/**
 * Freeze and thaw token account instructions.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';
import { getDiscriminatorOnlyEncoder } from '../codecs/instructions.js';

/**
 * Parameters for freezing a token account.
 */
export interface FreezeParams {
    /** Token account to freeze */
    tokenAccount: Address;
    /** Mint address */
    mint: Address;
    /** Freeze authority - must be signer */
    freezeAuthority: Address;
}

/**
 * Creates a freeze instruction (discriminator: 10).
 *
 * Freezes a token account, preventing transfers.
 *
 * @param params - Freeze parameters
 * @returns The freeze instruction
 */
export function createFreezeInstruction(params: FreezeParams): Instruction {
    const { tokenAccount, mint, freezeAuthority } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.READONLY },
        { address: freezeAuthority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({
            discriminator: DISCRIMINATOR.FREEZE,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

/**
 * Parameters for thawing a token account.
 */
export interface ThawParams {
    /** Token account to thaw */
    tokenAccount: Address;
    /** Mint address */
    mint: Address;
    /** Freeze authority - must be signer */
    freezeAuthority: Address;
}

/**
 * Creates a thaw instruction (discriminator: 11).
 *
 * Thaws a frozen token account, allowing transfers again.
 *
 * @param params - Thaw parameters
 * @returns The thaw instruction
 */
export function createThawInstruction(params: ThawParams): Instruction {
    const { tokenAccount, mint, freezeAuthority } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.READONLY },
        { address: freezeAuthority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({
            discriminator: DISCRIMINATOR.THAW,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
