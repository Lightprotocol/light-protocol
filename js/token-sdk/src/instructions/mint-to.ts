/**
 * Mint-to token instructions.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';
import { validatePositiveAmount, validateDecimals } from '../utils/validation.js';
import {
    getAmountInstructionEncoder,
    getCheckedInstructionEncoder,
} from '../codecs/instructions.js';

/**
 * Parameters for minting tokens.
 */
export interface MintToParams {
    /** Mint address */
    mint: Address;
    /** Token account to mint to */
    tokenAccount: Address;
    /** Mint authority - must be signer */
    mintAuthority: Address;
    /** Amount to mint */
    amount: bigint;
}

/**
 * Creates a mint-to instruction (discriminator: 7).
 *
 * Mints tokens to a decompressed CToken account.
 *
 * @param params - Mint-to parameters
 * @returns The mint-to instruction
 */
export function createMintToInstruction(params: MintToParams): Instruction {
    const { mint, tokenAccount, mintAuthority, amount } = params;

    validatePositiveAmount(amount);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: mint, role: AccountRole.WRITABLE },
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mintAuthority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data
    const data = new Uint8Array(
        getAmountInstructionEncoder().encode({
            discriminator: DISCRIMINATOR.MINT_TO,
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
 * Parameters for mint-to checked.
 */
export interface MintToCheckedParams extends MintToParams {
    /** Expected decimals */
    decimals: number;
}

/**
 * Creates a mint-to checked instruction (discriminator: 14).
 *
 * Mints tokens with decimals validation.
 *
 * @param params - Mint-to checked parameters
 * @returns The mint-to checked instruction
 */
export function createMintToCheckedInstruction(
    params: MintToCheckedParams,
): Instruction {
    const { mint, tokenAccount, mintAuthority, amount, decimals } = params;

    validatePositiveAmount(amount);
    validateDecimals(decimals);

    // Build accounts
    const accounts: AccountMeta[] = [
        { address: mint, role: AccountRole.WRITABLE },
        { address: tokenAccount, role: AccountRole.WRITABLE },
        { address: mintAuthority, role: AccountRole.READONLY_SIGNER },
    ];

    // Build instruction data
    const data = new Uint8Array(
        getCheckedInstructionEncoder().encode({
            discriminator: DISCRIMINATOR.MINT_TO_CHECKED,
            amount,
            decimals,
        }),
    );

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
