/**
 * Close token account action.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../../constants.js';

// ============================================================================
// CLOSE ACCOUNT INSTRUCTION
// ============================================================================

/**
 * Parameters for closing a token account.
 */
export interface CloseAccountParams {
    /** Token account to close */
    tokenAccount: Address;
    /** Destination for remaining lamports */
    destination: Address;
    /** Owner of the token account - must be signer */
    owner: Address;
}

/**
 * Creates a close token account instruction (discriminator: 9).
 *
 * Closes a decompressed CToken account and returns rent to the destination.
 * For compressible accounts, rent goes to the rent sponsor.
 *
 * @param params - Close account parameters
 * @returns The close instruction
 */
export function createCloseAccountInstruction(
    params: CloseAccountParams,
): IInstruction {
    const { tokenAccount, destination, owner } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: destination, role: 1 }, // writable
        { address: owner, role: 2 }, // readonly+signer
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array([DISCRIMINATOR.CLOSE]);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
