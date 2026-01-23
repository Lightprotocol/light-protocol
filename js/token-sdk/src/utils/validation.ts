/**
 * Validation utilities for Light Token accounts.
 */

import type { Address } from '@solana/addresses';
import { LIGHT_TOKEN_PROGRAM_ID } from '../constants.js';
import { deriveAssociatedTokenAddress } from './derivation.js';

// ============================================================================
// ACCOUNT TYPE DETECTION
// ============================================================================

/**
 * Checks if an account owner indicates a Light Token account.
 *
 * @param owner - The account owner address
 * @returns True if the owner is the Light Token program
 */
export function isLightTokenAccount(owner: Address): boolean {
    return owner === LIGHT_TOKEN_PROGRAM_ID;
}

/**
 * Determines the transfer type based on source and destination owners.
 *
 * @param sourceOwner - Owner of the source account
 * @param destOwner - Owner of the destination account
 * @returns The transfer type
 */
export function determineTransferType(
    sourceOwner: Address,
    destOwner: Address,
): 'light-to-light' | 'light-to-spl' | 'spl-to-light' | 'spl-to-spl' {
    const sourceIsLight = isLightTokenAccount(sourceOwner);
    const destIsLight = isLightTokenAccount(destOwner);

    if (sourceIsLight && destIsLight) {
        return 'light-to-light';
    }
    if (sourceIsLight && !destIsLight) {
        return 'light-to-spl';
    }
    if (!sourceIsLight && destIsLight) {
        return 'spl-to-light';
    }
    return 'spl-to-spl';
}

// ============================================================================
// ATA VALIDATION
// ============================================================================

/**
 * Validates that an ATA address matches the expected derivation.
 *
 * @param ata - The ATA address to validate
 * @param owner - The expected owner
 * @param mint - The expected mint
 * @returns True if the ATA matches the derivation
 */
export async function validateAtaDerivation(
    ata: Address,
    owner: Address,
    mint: Address,
): Promise<boolean> {
    const { address: derivedAta } = await deriveAssociatedTokenAddress(
        owner,
        mint,
    );
    return ata === derivedAta;
}

// ============================================================================
// AMOUNT VALIDATION
// ============================================================================

/**
 * Validates that a transfer amount is positive.
 *
 * @param amount - The amount to validate
 * @throws Error if amount is not positive
 */
export function validatePositiveAmount(amount: bigint): void {
    if (amount <= 0n) {
        throw new Error('Amount must be positive');
    }
}

/**
 * Validates decimal places for checked operations.
 *
 * @param decimals - The decimals value (0-255)
 * @throws Error if decimals is out of range
 */
export function validateDecimals(decimals: number): void {
    if (decimals < 0 || decimals > 255 || !Number.isInteger(decimals)) {
        throw new Error('Decimals must be an integer between 0 and 255');
    }
}
