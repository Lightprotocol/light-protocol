import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

/**
 * Get ATA program ID for a token program ID
 * @param tokenProgramId    Token program ID
 * @returns ATA program ID
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
 * Check if an ATA address matches the expected derivation from mint+owner.
 *
 * Pass programId for fast path.
 *
 * @param ata       ATA address to check
 * @param mint      Mint address
 * @param owner     Owner address
 * @param programId Optional: if known, only check this program's ATA
 * @returns         Result with detected type, or throws on mismatch
 */
export function checkAtaAddress(
    ata: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    programId?: PublicKey,
): AtaValidationResult {
    // fast path
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

    let ctokenExpected: PublicKey;
    let splExpected: PublicKey;
    let t22Expected: PublicKey;

    // c-token
    ctokenExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        CTOKEN_PROGRAM_ID,
        getAtaProgramId(CTOKEN_PROGRAM_ID),
    );
    if (ata.equals(ctokenExpected)) {
        return {
            valid: true,
            type: 'ctoken',
            programId: CTOKEN_PROGRAM_ID,
        };
    }

    // SPL
    splExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    if (ata.equals(splExpected)) {
        return { valid: true, type: 'spl', programId: TOKEN_PROGRAM_ID };
    }

    // T22
    t22Expected = getAssociatedTokenAddressSync(
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
