/**
 * Mint-to token instructions.
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
} from '../codecs/instructions.js';
import { buildInstructionDataWithMaxTopUp } from './helpers.js';

/**
 * Parameters for minting tokens.
 */
export interface MintToParams {
    /** Mint address (CMint) */
    mint: Address;
    /** Token account to mint to */
    tokenAccount: Address;
    /** Mint authority - must be signer */
    mintAuthority: Address;
    /** Amount to mint */
    amount: bigint;
    /** Maximum lamports for rent top-up (optional, 0 = no limit) */
    maxTopUp?: number;
    /** Fee payer for rent top-ups (optional, defaults to authority) */
    feePayer?: Address;
}

/**
 * Creates a mint-to instruction (discriminator: 7).
 *
 * Mints tokens to a decompressed CToken account.
 *
 * Account layout:
 * 0: CMint account (writable)
 * 1: destination CToken account (writable)
 * 2: authority (signer, writable unless feePayer provided)
 * 3: system_program (readonly)
 * 4: fee_payer (optional, signer, writable)
 *
 * @param params - Mint-to parameters
 * @returns The mint-to instruction
 */
export function createMintToInstruction(params: MintToParams): Instruction {
    const { mint, tokenAccount, mintAuthority, amount, maxTopUp, feePayer } =
        params;

    validatePositiveAmount(amount);

    const accounts: AccountMeta[] = [
        { address: mint, role: AccountRole.WRITABLE },
        { address: tokenAccount, role: AccountRole.WRITABLE },
        {
            address: mintAuthority,
            role: feePayer
                ? AccountRole.READONLY_SIGNER
                : AccountRole.WRITABLE_SIGNER,
        },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
    ];
    if (feePayer) {
        accounts.push({ address: feePayer, role: AccountRole.WRITABLE_SIGNER });
    }

    const baseBytes = getAmountInstructionEncoder().encode({
        discriminator: DISCRIMINATOR.MINT_TO,
        amount,
    });

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data: buildInstructionDataWithMaxTopUp(baseBytes, maxTopUp),
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
    const {
        mint,
        tokenAccount,
        mintAuthority,
        amount,
        decimals,
        maxTopUp,
        feePayer,
    } = params;

    validatePositiveAmount(amount);
    validateDecimals(decimals);

    const accounts: AccountMeta[] = [
        { address: mint, role: AccountRole.WRITABLE },
        { address: tokenAccount, role: AccountRole.WRITABLE },
        {
            address: mintAuthority,
            role: feePayer
                ? AccountRole.READONLY_SIGNER
                : AccountRole.WRITABLE_SIGNER,
        },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
    ];
    if (feePayer) {
        accounts.push({ address: feePayer, role: AccountRole.WRITABLE_SIGNER });
    }

    const baseBytes = getCheckedInstructionEncoder().encode({
        discriminator: DISCRIMINATOR.MINT_TO_CHECKED,
        amount,
        decimals,
    });

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data: buildInstructionDataWithMaxTopUp(baseBytes, maxTopUp),
    };
}
