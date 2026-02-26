/**
 * Burn token instructions.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import {
    DISCRIMINATOR,
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
} from '../constants.js';
import { validatePositiveAmount, validateDecimals } from '../utils/validation.js';
import {
    getAmountInstructionEncoder,
    getCheckedInstructionEncoder,
    encodeMaxTopUp,
} from '../codecs/instructions.js';

/**
 * Parameters for burning tokens.
 */
export interface BurnParams {
    /** Token account to burn from */
    tokenAccount: Address;
    /** Mint address (CMint) */
    mint: Address;
    /** Authority (owner or delegate) - must be signer */
    authority: Address;
    /** Amount to burn */
    amount: bigint;
    /** Maximum lamports for rent top-up (optional, 0 = no limit) */
    maxTopUp?: number;
    /** Fee payer for rent top-ups (optional, defaults to authority) */
    feePayer?: Address;
}

/**
 * Creates a burn instruction (discriminator: 8).
 *
 * Burns tokens from the token account and updates mint supply.
 *
 * Account layout:
 * 0: source CToken account (writable)
 * 1: CMint account (writable)
 * 2: authority (signer, writable unless feePayer provided)
 * 3: system_program (readonly)
 * 4: fee_payer (optional, signer, writable)
 *
 * @param params - Burn parameters
 * @returns The burn instruction
 */
export function createBurnInstruction(params: BurnParams): Instruction {
    const { tokenAccount, mint, authority, amount, maxTopUp, feePayer } =
        params;

    validatePositiveAmount(amount);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.WRITABLE },
        {
            address: authority,
            role: feePayer
                ? AccountRole.READONLY_SIGNER
                : AccountRole.WRITABLE_SIGNER,
        },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
    ];

    // Add fee payer if provided
    if (feePayer) {
        accounts.push({ address: feePayer, role: AccountRole.WRITABLE_SIGNER });
    }

    // Build instruction data: discriminator + amount [+ maxTopUp]
    const baseBytes = getAmountInstructionEncoder().encode({
        discriminator: DISCRIMINATOR.BURN,
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
): Instruction {
    const {
        tokenAccount,
        mint,
        authority,
        amount,
        decimals,
        maxTopUp,
        feePayer,
    } = params;

    validatePositiveAmount(amount);
    validateDecimals(decimals);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.WRITABLE },
        {
            address: authority,
            role: feePayer
                ? AccountRole.READONLY_SIGNER
                : AccountRole.WRITABLE_SIGNER,
        },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
    ];

    // Add fee payer if provided
    if (feePayer) {
        accounts.push({ address: feePayer, role: AccountRole.WRITABLE_SIGNER });
    }

    // Build instruction data: discriminator + amount + decimals [+ maxTopUp]
    const baseBytes = getCheckedInstructionEncoder().encode({
        discriminator: DISCRIMINATOR.BURN_CHECKED,
        amount,
        decimals,
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
