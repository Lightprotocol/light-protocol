/**
 * Approve and revoke delegate actions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';
import { getU64Encoder } from '@solana/codecs';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../../constants.js';

// ============================================================================
// APPROVE INSTRUCTION
// ============================================================================

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
export function createApproveInstruction(params: ApproveParams): IInstruction {
    const { tokenAccount, delegate, owner, amount } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: delegate, role: 0 }, // readonly
        { address: owner, role: 2 }, // readonly+signer
    ];

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length);
    data[0] = DISCRIMINATOR.APPROVE;
    data.set(new Uint8Array(amountBytes), 1);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}

// ============================================================================
// REVOKE INSTRUCTION
// ============================================================================

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
export function createRevokeInstruction(params: RevokeParams): IInstruction {
    const { tokenAccount, owner } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: tokenAccount, role: 1 }, // writable
        { address: owner, role: 2 }, // readonly+signer
    ];

    // Build instruction data (just discriminator)
    const data = new Uint8Array([DISCRIMINATOR.REVOKE]);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
