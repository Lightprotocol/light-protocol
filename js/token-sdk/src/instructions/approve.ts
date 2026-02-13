/**
 * Approve and revoke delegate instructions.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';
import { validatePositiveAmount } from '../utils/validation.js';
import {
    getAmountInstructionEncoder,
    getDiscriminatorOnlyEncoder,
} from '../codecs/instructions.js';

/**
 * Parameters for approving a delegate.
 */
export interface ApproveParams {
    /** Token account to approve delegate on */
    tokenAccount: Address;
    /** Delegate to approve */
    delegate: Address;
    /** Owner of the token account - must be signer */
    owner: Address;
    /** Amount to delegate */
    amount: bigint;
}

/**
 * Creates an approve instruction (discriminator: 4).
 *
 * Approves a delegate to transfer up to the specified amount.
 *
 * @param params - Approve parameters
 * @returns The approve instruction
 */
export function createApproveInstruction(params: ApproveParams): Instruction {
    const { tokenAccount, delegate, owner, amount } = params;

    validatePositiveAmount(amount);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: delegate, role: AccountRole.READONLY },
        { address: owner, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data
    const data = new Uint8Array(
        getAmountInstructionEncoder().encode({
            discriminator: DISCRIMINATOR.APPROVE,
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
 * Parameters for revoking a delegate.
 */
export interface RevokeParams {
    /** Token account to revoke delegate from */
    tokenAccount: Address;
    /** Owner of the token account - must be signer */
    owner: Address;
}

/**
 * Creates a revoke instruction (discriminator: 5).
 *
 * Revokes the delegate authority from the token account.
 *
 * @param params - Revoke parameters
 * @returns The revoke instruction
 */
export function createRevokeInstruction(params: RevokeParams): Instruction {
    const { tokenAccount, owner } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: owner, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array(
        getDiscriminatorOnlyEncoder().encode({
            discriminator: DISCRIMINATOR.REVOKE,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
