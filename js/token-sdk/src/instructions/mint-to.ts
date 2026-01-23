/**
 * Mint-to token instructions.
 */

import type { Address } from '@solana/addresses';
import type { IInstruction, IAccountMeta } from '@solana/instructions';
import { getU64Encoder } from '@solana/codecs';

import { DISCRIMINATOR, LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';

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
export function createMintToInstruction(params: MintToParams): IInstruction {
    const { mint, tokenAccount, mintAuthority, amount } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: mint, role: 1 }, // writable
        { address: tokenAccount, role: 1 }, // writable
        { address: mintAuthority, role: 2 }, // readonly+signer
    ];

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length);
    data[0] = DISCRIMINATOR.MINT_TO;
    data.set(new Uint8Array(amountBytes), 1);

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
): IInstruction {
    const { mint, tokenAccount, mintAuthority, amount, decimals } = params;

    // Build accounts
    const accounts: IAccountMeta[] = [
        { address: mint, role: 1 }, // writable
        { address: tokenAccount, role: 1 }, // writable
        { address: mintAuthority, role: 2 }, // readonly+signer
    ];

    // Build instruction data
    const amountBytes = getU64Encoder().encode(amount);
    const data = new Uint8Array(1 + amountBytes.length + 1);
    data[0] = DISCRIMINATOR.MINT_TO_CHECKED;
    data.set(new Uint8Array(amountBytes), 1);
    data[1 + amountBytes.length] = decimals;

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
