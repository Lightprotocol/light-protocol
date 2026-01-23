/**
 * Burn token actions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';
import { getU64Encoder } from '@solana/codecs';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../../constants.js';

// ============================================================================
// BURN INSTRUCTION
// ============================================================================

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
export function createBurnInstruction(params: BurnParams): IInstruction {
    const { tokenAccount, mint, authority, amount } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: mint, role: 1 }, // writable
        { address: authority, role: 2 }, // readonly+signer
    ];

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length);
    data[0] = DISCRIMINATOR.BURN;
    data.set(new Uint8Array(amountBytes), 1);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

// ============================================================================
// BURN CHECKED INSTRUCTION
// ============================================================================

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
): IInstruction {
    const { tokenAccount, mint, authority, amount, decimals } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: mint, role: 1 }, // writable
        { address: authority, role: 2 }, // readonly+signer
    ];

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length + 1);
    data[0] = DISCRIMINATOR.BURN_CHECKED;
    data.set(new Uint8Array(amountBytes), 1);
    data[1 + amountBytes.length] = decimals;

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
