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
    /** Rent sponsor for compressible accounts (optional, writable) */
    rentSponsor?: Address;
}

/**
 * Creates a close token account instruction (discriminator: 9).
 *
 * Closes a decompressed CToken account and returns rent to the destination.
 * For compressible accounts, rent goes to the rent sponsor.
 *
 * Account layout:
 * 0: token account (writable)
 * 1: destination (writable)
 * 2: authority/owner (signer)
 * 3: rent_sponsor (optional, writable) - required for compressible accounts
 *
 * @param params - Close account parameters
 * @returns The close instruction
 */
export function createCloseAccountInstruction(
    params: CloseAccountParams,
): Instruction {
    const { tokenAccount, destination, owner, rentSponsor } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: destination, role: AccountRole.WRITABLE },
        { address: owner, role: AccountRole.READONLY_SIGNER },
    ];

    // Add rent sponsor if provided (required for compressible accounts)
    if (rentSponsor) {
        accounts.push({ address: rentSponsor, role: AccountRole.WRITABLE });
    }

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
