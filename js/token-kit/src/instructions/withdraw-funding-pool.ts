/**
 * Withdraw from funding pool instruction.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import { getU64Encoder } from '@solana/codecs';

import {
    DISCRIMINATOR,
    LIGHT_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
} from '../constants.js';

/**
 * Parameters for withdrawing from a funding pool.
 */
export interface WithdrawFundingPoolParams {
    /** Rent sponsor pool PDA (writable) */
    rentSponsor: Address;
    /** Compression authority (signer) */
    compressionAuthority: Address;
    /** Destination account receiving withdrawn lamports (writable) */
    destination: Address;
    /** Compressible config account (readonly) */
    compressibleConfig: Address;
    /** Amount of lamports to withdraw */
    amount: bigint;
}

/**
 * Creates a withdraw funding pool instruction (discriminator: 105).
 *
 * Withdraws lamports from the rent sponsor funding pool.
 *
 * Account layout:
 * 0: rent_sponsor (writable) - Pool PDA
 * 1: compression_authority (signer)
 * 2: destination (writable) - Receives withdrawn lamports
 * 3: system_program (readonly)
 * 4: compressible_config (readonly)
 *
 * @param params - Withdraw funding pool parameters
 * @returns The withdraw funding pool instruction
 */
export function createWithdrawFundingPoolInstruction(
    params: WithdrawFundingPoolParams,
): Instruction {
    const {
        rentSponsor,
        compressionAuthority,
        destination,
        compressibleConfig,
        amount,
    } = params;

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: rentSponsor, role: AccountRole.WRITABLE },
        { address: compressionAuthority, role: AccountRole.READONLY_SIGNER },
        { address: destination, role: AccountRole.WRITABLE },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        { address: compressibleConfig, role: AccountRole.READONLY },
    ];

    // Build instruction data: discriminator (u8) + amount (u64)
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length);
    data[0] = DISCRIMINATOR.WITHDRAW_FUNDING_POOL;
    data.set(new Uint8Array(amountBytes), 1);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
