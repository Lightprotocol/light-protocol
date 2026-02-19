import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    Transaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    createAssociatedTokenAccountInterfaceInstruction,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    CTokenConfig,
} from '../instructions/create-ata-interface';
import { getAtaProgramId } from '../ata-utils';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';

// Re-export types for backwards compatibility
export type { CTokenConfig };

/**
 * Create an associated token account for SPL/T22/light-token. Defaults to
 * light-token program.
 *
 * @param rpc                       RPC connection
 * @param payer                     Fee payer and transaction signer
 * @param mint                      Mint address
 * @param owner                     Owner of the associated token account
 * @param allowOwnerOffCurve        Allow owner to be a PDA (default: false)
 * @param confirmOptions            Options for confirming the transaction
 * @param programId                 Token program ID (default:
 *                                  LIGHT_TOKEN_PROGRAM_ID)
 * @param associatedTokenProgramId  associated token account program ID
 *                                  (auto-derived if not provided)
 * @param ctokenConfig              Optional rent config
 * @returns Address of the new associated token account
 */
export async function createAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): Promise<PublicKey> {
    assertBetaEnabled();

    const effectiveAtaProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    const associatedToken = getAssociatedTokenAddressInterface(
        mint,
        owner,
        allowOwnerOffCurve,
        programId,
        effectiveAtaProgramId,
    );

    const ix = createAssociatedTokenAccountInterfaceInstruction(
        payer.publicKey,
        associatedToken,
        owner,
        mint,
        programId,
        effectiveAtaProgramId,
        ctokenConfig,
    );

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 30_000 }), ix],
            payer,
            blockhash,
            [],
        );
        await sendAndConfirmTx(rpc, tx, confirmOptions);
    } else {
        const transaction = new Transaction().add(ix);
        await sendAndConfirmTransaction(
            rpc,
            transaction,
            [payer],
            confirmOptions,
        );
    }

    return associatedToken;
}

/**
 * Create an associated token account idempotently for SPL/T22/light-token.
 * Defaults to light-token program.
 *
 * If the account already exists, the instruction succeeds without error.
 *
 * @param rpc                       RPC connection
 * @param payer                     Fee payer and transaction signer
 * @param mint                      Mint address
 * @param owner                     Owner of the associated token account
 * @param allowOwnerOffCurve        Allow owner to be a PDA (default: false)
 * @param confirmOptions            Options for confirming the transaction
 * @param programId                 Token program ID (default:
 *                                  LIGHT_TOKEN_PROGRAM_ID)
 * @param associatedTokenProgramId  associated token account program ID
 *                                  (auto-derived if not provided)
 * @param ctokenConfig              Optional light-token-specific configuration
 *
 * @returns Address of the associated token account
 */
export async function createAtaInterfaceIdempotent(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): Promise<PublicKey> {
    assertBetaEnabled();

    const effectiveAtaProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    const associatedToken = getAssociatedTokenAddressInterface(
        mint,
        owner,
        allowOwnerOffCurve,
        programId,
        effectiveAtaProgramId,
    );

    const ix = createAssociatedTokenAccountInterfaceIdempotentInstruction(
        payer.publicKey,
        associatedToken,
        owner,
        mint,
        programId,
        effectiveAtaProgramId,
        ctokenConfig,
    );

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 30_000 }), ix],
            payer,
            blockhash,
            [],
        );
        await sendAndConfirmTx(rpc, tx, confirmOptions);
    } else {
        const transaction = new Transaction().add(ix);
        await sendAndConfirmTransaction(
            rpc,
            transaction,
            [payer],
            confirmOptions,
        );
    }

    return associatedToken;
}
