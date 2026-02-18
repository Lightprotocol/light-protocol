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
    encodeMaxTopUp,
} from '../codecs/instructions.js';

/**
 * Parameters for approving a delegate.
 */
export interface ApproveParams {
    /** Token account to approve delegate on */
    tokenAccount: Address;
    /** Delegate to approve */
    delegate: Address;
    /** Owner of the token account - must be signer and payer */
    owner: Address;
    /** Amount to delegate */
    amount: bigint;
    /** Maximum lamports for rent top-up in units of 1,000 lamports (optional) */
    maxTopUp?: number;
}

/**
 * Creates an approve instruction (discriminator: 4).
 *
 * Approves a delegate to transfer up to the specified amount.
 *
 * Account layout:
 * 0: token account (writable)
 * 1: delegate (readonly)
 * 2: owner (signer, writable) - always the payer (APPROVE_PAYER_IDX=2 in Rust)
 *
 * Note: Unlike transfer/burn/mint-to, approve does NOT support a separate fee payer.
 * The owner is always the payer for compressible rent top-ups.
 *
 * @param params - Approve parameters
 * @returns The approve instruction
 */
export function createApproveInstruction(params: ApproveParams): Instruction {
    const { tokenAccount, delegate, owner, amount, maxTopUp } = params;

    validatePositiveAmount(amount);

    // Build accounts - owner is always WRITABLE_SIGNER (payer at index 2)
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: delegate, role: AccountRole.READONLY },
        { address: owner, role: AccountRole.WRITABLE_SIGNER },
    ];

    // Build instruction data: discriminator + amount [+ maxTopUp]
    const baseBytes = getAmountInstructionEncoder().encode({
        discriminator: DISCRIMINATOR.APPROVE,
        amount,
    });
    const maxTopUpBytes = encodeMaxTopUp(maxTopUp);

    const data = new Uint8Array(baseBytes.length + maxTopUpBytes.length);
    data.set(new Uint8Array(baseBytes), 0);
    if (maxTopUpBytes.length > 0) {
        data.set(maxTopUpBytes, baseBytes.length);
    }

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
    /** Owner of the token account - must be signer and payer */
    owner: Address;
    /** Maximum lamports for rent top-up in units of 1,000 lamports (optional) */
    maxTopUp?: number;
}

/**
 * Creates a revoke instruction (discriminator: 5).
 *
 * Revokes the delegate authority from the token account.
 *
 * Account layout:
 * 0: token account (writable)
 * 1: owner (signer, writable) - always the payer (REVOKE_PAYER_IDX=1 in Rust)
 *
 * Note: Unlike transfer/burn/mint-to, revoke does NOT support a separate fee payer.
 * The owner is always the payer for compressible rent top-ups.
 *
 * @param params - Revoke parameters
 * @returns The revoke instruction
 */
export function createRevokeInstruction(params: RevokeParams): Instruction {
    const { tokenAccount, owner, maxTopUp } = params;

    // Build accounts - owner is always WRITABLE_SIGNER (payer at index 1)
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: owner, role: AccountRole.WRITABLE_SIGNER },
    ];

    // Build instruction data: discriminator [+ maxTopUp]
    const baseBytes = getDiscriminatorOnlyEncoder().encode({
        discriminator: DISCRIMINATOR.REVOKE,
    });
    const maxTopUpBytes = encodeMaxTopUp(maxTopUp);

    const data = new Uint8Array(baseBytes.length + maxTopUpBytes.length);
    data.set(new Uint8Array(baseBytes), 0);
    if (maxTopUpBytes.length > 0) {
        data.set(maxTopUpBytes, baseBytes.length);
    }

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
