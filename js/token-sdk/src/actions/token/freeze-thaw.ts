/**
 * Freeze and thaw token account actions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../../constants.js';

// ============================================================================
// FREEZE INSTRUCTION
// ============================================================================

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
export function createFreezeInstruction(params: FreezeParams): IInstruction {
    const { tokenAccount, mint, freezeAuthority } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: mint, role: 0 }, // readonly
        { address: freezeAuthority, role: 2 }, // readonly+signer
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array([DISCRIMINATOR.FREEZE]);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

// ============================================================================
// THAW INSTRUCTION
// ============================================================================

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
export function createThawInstruction(params: ThawParams): IInstruction {
    const { tokenAccount, mint, freezeAuthority } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: mint, role: 0 }, // readonly
        { address: freezeAuthority, role: 2 }, // readonly+signer
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array([DISCRIMINATOR.THAW]);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
