/**
 * CToken transfer instructions.
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
 * Parameters for CToken transfer.
 */
export interface TransferParams {
    /** Source CToken account */
    source: Address;
    /** Destination CToken account */
    destination: Address;
    /** Amount to transfer */
    amount: bigint;
    /** Authority (owner or delegate) - must be signer */
    authority: Address;
    /** Maximum lamports for rent top-up (optional, 0 = no limit) */
    maxTopUp?: number;
    /** Fee payer for rent top-ups (optional, defaults to authority) */
    feePayer?: Address;
}

/**
 * Creates a CToken transfer instruction (discriminator: 3).
 *
 * Transfers tokens between decompressed CToken accounts.
 *
 * @param params - Transfer parameters
 * @returns The transfer instruction
 */
export function createTransferInstruction(
    params: TransferParams,
): Instruction {
    const { source, destination, amount, authority, maxTopUp, feePayer } =
        params;

    validatePositiveAmount(amount);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: source, role: AccountRole.WRITABLE },
        { address: destination, role: AccountRole.WRITABLE },
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
        discriminator: DISCRIMINATOR.TRANSFER,
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
 * Parameters for CToken transfer checked.
 */
export interface TransferCheckedParams extends TransferParams {
    /** Mint address for validation */
    mint: Address;
    /** Expected decimals */
    decimals: number;
}

/**
 * Creates a CToken transfer checked instruction (discriminator: 12).
 *
 * Transfers tokens with decimals validation.
 *
 * @param params - Transfer checked parameters
 * @returns The transfer checked instruction
 */
export function createTransferCheckedInstruction(
    params: TransferCheckedParams,
): Instruction {
    const {
        source,
        mint,
        destination,
        amount,
        authority,
        decimals,
        maxTopUp,
        feePayer,
    } = params;

    validatePositiveAmount(amount);
    validateDecimals(decimals);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: source, role: AccountRole.WRITABLE },
        { address: mint, role: AccountRole.READONLY },
        { address: destination, role: AccountRole.WRITABLE },
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
        discriminator: DISCRIMINATOR.TRANSFER_CHECKED,
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
