import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction as createSplAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction as createSplAssociatedTokenAccountIdempotentInstruction,
} from '@solana/spl-token';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getAtaProgramId } from '../ata-utils';
import {
    createAssociatedCTokenAccountInstruction,
    createAssociatedCTokenAccountIdempotentInstruction,
    CompressibleConfig,
    DEFAULT_COMPRESSIBLE_CONFIG,
} from './create-associated-ctoken';

// Re-export for convenience
export { DEFAULT_COMPRESSIBLE_CONFIG };

/**
 * c-token-specific config for createAssociatedTokenAccountInterfaceInstruction
 */
export interface CTokenConfig {
    compressibleConfig?: CompressibleConfig;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

// Keep old interface type for backwards compatibility export
export interface CreateAssociatedTokenAccountInterfaceInstructionParams {
    payer: PublicKey;
    associatedToken: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    programId?: PublicKey;
    associatedTokenProgramId?: PublicKey;
    compressibleConfig?: CompressibleConfig;
    configAccount?: PublicKey;
    rentPayerPda?: PublicKey;
}

/**
 * Create instruction for creating an associated token account (SPL, Token-2022,
 * or c-token). Follows SPL Token API signature with optional c-token config at the
 * end.
 *
 * @param payer                    Fee payer public key.
 * @param associatedToken          Associated token account address.
 * @param owner                    Owner of the associated token account.
 * @param mint                     Mint address.
 * @param programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param associatedTokenProgramId Associated token program ID.
 * @param ctokenConfig             Optional c-token-specific configuration.
 */
export function createAssociatedTokenAccountInterfaceInstruction(
    payer: PublicKey,
    associatedToken: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): TransactionInstruction {
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        return createAssociatedCTokenAccountInstruction(
            payer,
            owner,
            mint,
            ctokenConfig?.compressibleConfig,
            ctokenConfig?.configAccount,
            ctokenConfig?.rentPayerPda,
        );
    } else {
        return createSplAssociatedTokenAccountInstruction(
            payer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

/**
 * Create idempotent instruction for creating an associated token account (SPL,
 * Token-2022, or c-token). Follows SPL Token API signature with optional c-token
 * config at the end.
 *
 * @param payer                    Fee payer public key.
 * @param associatedToken          Associated token account address.
 * @param owner                    Owner of the associated token account.
 * @param mint                     Mint address.
 * @param programId                Token program ID (default: TOKEN_PROGRAM_ID).
 * @param associatedTokenProgramId Associated token program ID.
 * @param ctokenConfig             Optional c-token-specific configuration.
 */
export function createAssociatedTokenAccountInterfaceIdempotentInstruction(
    payer: PublicKey,
    associatedToken: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    programId: PublicKey = TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): TransactionInstruction {
    const effectiveAssociatedTokenProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        return createAssociatedCTokenAccountIdempotentInstruction(
            payer,
            owner,
            mint,
            ctokenConfig?.compressibleConfig,
            ctokenConfig?.configAccount,
            ctokenConfig?.rentPayerPda,
        );
    } else {
        return createSplAssociatedTokenAccountIdempotentInstruction(
            payer,
            associatedToken,
            owner,
            mint,
            programId,
            effectiveAssociatedTokenProgramId,
        );
    }
}

/**
 * Short alias for createAssociatedTokenAccountInterfaceIdempotentInstruction.
 */
export const createAtaInterfaceIdempotentInstruction =
    createAssociatedTokenAccountInterfaceIdempotentInstruction;
