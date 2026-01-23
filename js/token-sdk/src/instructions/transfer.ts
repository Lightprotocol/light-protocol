/**
 * CToken transfer instructions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';
import { getU64Encoder, getU16Encoder } from '@solana/codecs';

import {
    DISCRIMINATOR,
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
} from '../constants.js';

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
): IInstruction {
    const { source, destination, amount, authority, maxTopUp, feePayer } =
        params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: source, role: 1 }, // writable
        { address: destination, role: 1 }, // writable
        {
            address: authority,
            role: maxTopUp !== undefined && !feePayer ? 3 : 2, // writable+signer if paying, else readonly+signer
        },
        { address: SYSTEM_PROGRAM_ID, role: 0 }, // readonly
    ];

    // Add fee payer if provided
    if (feePayer) {
        accounts.push({ address: feePayer, role: 3 }); // writable+signer
    }

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const maxTopUpBytes =
        maxTopUp !== undefined
            ? getU16Encoder().encode(maxTopUp)
            : new Uint8Array(0);

    const data = new Uint8Array(1 + amountBytes.length + maxTopUpBytes.length);
    data[0] = DISCRIMINATOR.TRANSFER;
    data.set(new Uint8Array(amountBytes), 1);
    if (maxTopUpBytes.length > 0) {
        data.set(new Uint8Array(maxTopUpBytes), 1 + amountBytes.length);
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
): IInstruction {
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

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: source, role: 1 }, // writable
        { address: mint, role: 0 }, // readonly
        { address: destination, role: 1 }, // writable
        {
            address: authority,
            role: maxTopUp !== undefined && !feePayer ? 3 : 2, // writable+signer if paying, else readonly+signer
        },
        { address: SYSTEM_PROGRAM_ID, role: 0 }, // readonly
    ];

    // Add fee payer if provided
    if (feePayer) {
        accounts.push({ address: feePayer, role: 3 }); // writable+signer
    }

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const maxTopUpBytes =
        maxTopUp !== undefined
            ? getU16Encoder().encode(maxTopUp)
            : new Uint8Array(0);

    const data = new Uint8Array(
        1 + amountBytes.length + 1 + maxTopUpBytes.length,
    );
    data[0] = DISCRIMINATOR.TRANSFER_CHECKED;
    data.set(new Uint8Array(amountBytes), 1);
    data[1 + amountBytes.length] = decimals;
    if (maxTopUpBytes.length > 0) {
        data.set(new Uint8Array(maxTopUpBytes), 1 + amountBytes.length + 1);
    }

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
