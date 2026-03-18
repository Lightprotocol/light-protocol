/**
 * Claim rent instruction.
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
 * Parameters for claiming rent from compressible accounts.
 */
export interface ClaimParams {
    /** Rent sponsor PDA receiving claimed rent (writable) */
    rentSponsor: Address;
    /** Compression authority (signer) */
    compressionAuthority: Address;
    /** Compressible config account (readonly) */
    compressibleConfig: Address;
    /** Token accounts to claim rent from (writable, variable count) */
    tokenAccounts: Address[];
}

/**
 * Creates a claim instruction (discriminator: 104).
 *
 * Claims rent from compressible token accounts and returns it to the
 * rent sponsor PDA.
 *
 * Account layout:
 * 0: rent_sponsor (writable) - PDA receiving claimed rent
 * 1: compression_authority (signer)
 * 2: compressible_config (readonly) - CompressibleConfig
 * 3+: token_accounts... (writable, variable count)
 *
 * @param params - Claim parameters
 * @returns The claim instruction
 */
export function createClaimInstruction(params: ClaimParams): Instruction {
    const { rentSponsor, compressionAuthority, compressibleConfig, tokenAccounts } =
        params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: rentSponsor, role: AccountRole.WRITABLE },
        { address: compressionAuthority, role: AccountRole.READONLY_SIGNER },
        { address: compressibleConfig, role: AccountRole.READONLY },
    ];

    // Add variable-count token accounts
    for (const tokenAccount of tokenAccounts) {
        accounts.push({ address: tokenAccount, role: AccountRole.WRITABLE });
    }

    // Build instruction data (just discriminator, no additional data)
    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({
            discriminator: DISCRIMINATOR.CLAIM,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
