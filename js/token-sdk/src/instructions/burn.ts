/**
 * Burn token instructions.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';
import { validatePositiveAmount, validateDecimals } from '../utils/validation.js';
import {
    getAmountInstructionEncoder,
    getCheckedInstructionEncoder,
} from '../codecs/instructions.js';

/**
 * Parameters for burning tokens.
 */
export interface BurnParams {
    /** Token account to burn from */
    tokenAccount: Address;
    /** Mint address */
    mint: Address;
    /** Authority (owner or delegate) - must be signer */
    authority: Address;
    /** Amount to burn */
    amount: bigint;
}

/**
 * Creates a burn instruction (discriminator: 8).
 *
 * Burns tokens from the token account and updates mint supply.
 *
 * @param params - Burn parameters
 * @returns The burn instruction
 */
export function createBurnInstruction(params: BurnParams): Instruction {
    const { tokenAccount, mint, authority, amount } = params;

    validatePositiveAmount(amount);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.WRITABLE },
        { address: authority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data
    const data = new Uint8Array(
        getAmountInstructionEncoder().encode({
            discriminator: DISCRIMINATOR.BURN,
            amount,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

/**
 * Parameters for burn checked.
 */
export interface BurnCheckedParams extends BurnParams {
    /** Expected decimals */
    decimals: number;
}

/**
 * Creates a burn checked instruction (discriminator: 15).
 *
 * Burns tokens with decimals validation.
 *
 * @param params - Burn checked parameters
 * @returns The burn checked instruction
 */
export function createBurnCheckedInstruction(
    params: BurnCheckedParams,
): Instruction {
    const { tokenAccount, mint, authority, amount, decimals } = params;

    validatePositiveAmount(amount);
    validateDecimals(decimals);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.WRITABLE },
        { address: authority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data
    const data = new Uint8Array(
        getCheckedInstructionEncoder().encode({
            discriminator: DISCRIMINATOR.BURN_CHECKED,
            amount,
            decimals,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
