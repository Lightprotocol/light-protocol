import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

export function getAtaProgramId(tokenProgramId: PublicKey): PublicKey {
    if (tokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return LIGHT_TOKEN_PROGRAM_ID;
    }
    return ASSOCIATED_TOKEN_PROGRAM_ID;
}

export type AtaType = 'spl' | 'token2022' | 'light-token';

export interface AtaValidationResult {
    valid: true;
    type: AtaType;
    programId: PublicKey;
}

export function checkAtaAddress(
    ata: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    programId?: PublicKey,
    allowOwnerOffCurve = false,
): AtaValidationResult {
    if (programId) {
        const expected = getAssociatedTokenAddressSync(
            mint,
            owner,
            allowOwnerOffCurve,
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

    const lightTokenExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        allowOwnerOffCurve,
        LIGHT_TOKEN_PROGRAM_ID,
        getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
    );
    if (ata.equals(lightTokenExpected)) {
        return {
            valid: true,
            type: 'light-token',
            programId: LIGHT_TOKEN_PROGRAM_ID,
        };
    }

    const splExpected = getAssociatedTokenAddressSync(
        mint,
        owner,
        allowOwnerOffCurve,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    if (ata.equals(splExpected)) {
        return { valid: true, type: 'spl', programId: TOKEN_PROGRAM_ID };
    }

    const t22Expected = getAssociatedTokenAddressSync(
        mint,
        owner,
        allowOwnerOffCurve,
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
            `light-token=${lightTokenExpected.toBase58()}, ` +
            `SPL=${splExpected.toBase58()}, ` +
            `T22=${t22Expected.toBase58()}`,
    );
}

function programIdToAtaType(programId: PublicKey): AtaType {
    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) return 'light-token';
    if (programId.equals(TOKEN_PROGRAM_ID)) return 'spl';
    if (programId.equals(TOKEN_2022_PROGRAM_ID)) return 'token2022';
    throw new Error(`Unknown program ID: ${programId.toBase58()}`);
}
