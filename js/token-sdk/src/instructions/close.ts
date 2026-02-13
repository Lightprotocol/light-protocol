/**
 * Close token account instruction.
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
): Instruction {
    const { tokenAccount, destination, owner } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: destination, role: AccountRole.WRITABLE },
        { address: owner, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({
            discriminator: DISCRIMINATOR.CLOSE,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
