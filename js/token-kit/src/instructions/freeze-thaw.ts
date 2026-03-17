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
 * Parameters for freezing or thawing a token account.
 */
export interface FreezeThawParams {
    /** Token account to freeze/thaw */
    tokenAccount: Address;
    /** Mint address */
    mint: Address;
    /** Freeze authority - must be signer */
    freezeAuthority: Address;
}

/** @deprecated Use FreezeThawParams instead. */
export type FreezeParams = FreezeThawParams;
/** @deprecated Use FreezeThawParams instead. */
export type ThawParams = FreezeThawParams;

function createFreezeThawInstruction(
    params: FreezeThawParams,
    discriminator: number,
): Instruction {
    const { tokenAccount, mint, freezeAuthority } = params;

    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.READONLY },
        { address: freezeAuthority, role: AccountRole.READONLY_SIGNER },
    ];

    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({ discriminator }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

/**
 * Creates a freeze instruction (discriminator: 10).
 *
 * Freezes a token account, preventing transfers.
 */
export function createFreezeInstruction(params: FreezeThawParams): Instruction {
    return createFreezeThawInstruction(params, DISCRIMINATOR.FREEZE);
}

/**
 * Creates a thaw instruction (discriminator: 11).
 *
 * Thaws a frozen token account, allowing transfers again.
 */
export function createThawInstruction(params: FreezeThawParams): Instruction {
    return createFreezeThawInstruction(params, DISCRIMINATOR.THAW);
}
