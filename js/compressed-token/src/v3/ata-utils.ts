import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

/**
 * Get the appropriate ATA program ID for a given token program ID
 * @param tokenProgramId - The token program ID
 * @returns The associated token program ID
 */
export function getAtaProgramId(tokenProgramId: PublicKey): PublicKey {
    if (tokenProgramId.equals(CTOKEN_PROGRAM_ID)) {
        return CTOKEN_PROGRAM_ID;
    }
    return ASSOCIATED_TOKEN_PROGRAM_ID;
}

/** ATA type for validation result */
export type AtaType = 'spl' | 'token2022' | 'ctoken';

/** Result of ATA validation */
export interface AtaValidationResult {
    valid: true;
    type: AtaType;
    programId: PublicKey;
}

/**
 * Validate that an ATA address matches the expected derivation from mint+owner.
 *
 * Performance: If programId is provided, only derives and checks that one ATA.
 * Otherwise derives all three (SPL, T22, c-token) until a match is found.
 *
 * @param ata       The ATA address to validate
 * @param mint      Mint address
 * @param owner     Owner address
 * @param programId Optional: if known, only check this program's ATA
 * @returns         Validation result with detected type, or throws on mismatch
 */
export function validateAtaAddress(
    ata: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    programId?: PublicKey,
): AtaValidationResult {
    // Hot path: programId specified - only check that one
    if (programId) {
        const expected = getAssociatedTokenAddressSync(
            mint,
            owner,
            false,
            programId,
            getAtaProgramId(programId),
        );
        if (ata.equals(expected)) {
            return {
                valid: true,
                type: programIdToAtaType(programId),
                programId,
            };
        }
        throw new Error(
            `ATA address mismatch for ${programId.toBase58()}. ` +
                `Expected: ${expected.toBase58()}, got: ${ata.toBase58()}`,
        );
    }

    // Check c-token first (most common for this codebase)
    const ctokenExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        CTOKEN_PROGRAM_ID,
        getAtaProgramId(CTOKEN_PROGRAM_ID),
    );
    if (ata.equals(ctokenExpected)) {
        return { valid: true, type: 'ctoken', programId: CTOKEN_PROGRAM_ID };
    }

    // Check SPL
    const splExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    if (ata.equals(splExpected)) {
        return { valid: true, type: 'spl', programId: TOKEN_PROGRAM_ID };
    }

    // Check T22
    const t22Expected = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_2022_PROGRAM_ID,
        getAtaProgramId(TOKEN_2022_PROGRAM_ID),
    );
    if (ata.equals(t22Expected)) {
        return {
            valid: true,
            type: 'token2022',
            programId: TOKEN_2022_PROGRAM_ID,
        };
    }

    // No match - invalid ATA
    throw new Error(
        `ATA address does not match any valid derivation from mint+owner. ` +
            `Got: ${ata.toBase58()}, expected one of: ` +
            `c-token=${ctokenExpected.toBase58()}, ` +
            `SPL=${splExpected.toBase58()}, ` +
            `T22=${t22Expected.toBase58()}`,
    );
}

/**
 * Convert programId to AtaType
 */
function programIdToAtaType(programId: PublicKey): AtaType {
    if (programId.equals(CTOKEN_PROGRAM_ID)) return 'ctoken';
    if (programId.equals(TOKEN_PROGRAM_ID)) return 'spl';
    if (programId.equals(TOKEN_2022_PROGRAM_ID)) return 'token2022';
    throw new Error(`Unknown program ID: ${programId.toBase58()}`);
}
