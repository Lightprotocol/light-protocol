import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    Transaction,
    TransactionSignature,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
    Rpc,
    CTOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import {
    createAssociatedTokenAccountInterfaceInstruction,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    CTokenConfig,
} from '../instructions/create-ata-interface';
import { getAtaProgramId } from '../ata-utils';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';

// Re-export types for backwards compatibility
export type { CTokenConfig };

// Keep old interface type for backwards compatibility export
export interface CreateAtaInterfaceParams {
    rpc: Rpc;
    payer: Signer;
    owner: PublicKey;
    mint: PublicKey;
    allowOwnerOffCurve?: boolean;
    confirmOptions?: ConfirmOptions;
    programId?: PublicKey;
    associatedTokenProgramId?: PublicKey;
    ctokenConfig?: CTokenConfig;
}

export interface CreateAtaInterfaceResult {
    address: PublicKey;
    transactionSignature: TransactionSignature;
}

/**
 * Create an associated token account for SPL Token, Token-2022, or Compressed Token.
 * Follows SPL Token createAssociatedTokenAccount signature.
 * Defaults to c-token program.
 *
 * Dispatches to the appropriate program based on `programId`:
 * - `CTOKEN_PROGRAM_ID` -> Compressed Token ATA (default)
 * - `TOKEN_PROGRAM_ID` -> SPL Token ATA
 * - `TOKEN_2022_PROGRAM_ID` -> Token-2022 ATA
 *
 * @param rpc                      RPC connection
 * @param payer                    Fee payer and transaction signer
 * @param mint                     Mint address
 * @param owner                    Owner of the associated token account
 * @param allowOwnerOffCurve       Allow owner to be a PDA (default: false)
 * @param confirmOptions           Options for confirming the transaction
 * @param programId                Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param associatedTokenProgramId Associated token program ID (auto-derived if not provided)
 * @param ctokenConfig             Optional c-token-specific configuration
 *
 * @example
 * // Create Compressed Token ATA (default)
 * const { address } = await createAtaInterface(
 *     rpc,
 *     payer,
 *     mint,
 *     wallet.publicKey,
 * );
 *
 * @example
 * // Create SPL Token ATA
 * const { address } = await createAtaInterface(
 *     rpc,
 *     payer,
 *     splMint,
 *     wallet.publicKey,
 *     false,
 *     undefined,
 *     TOKEN_PROGRAM_ID,
 * );
 */
export async function createAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): Promise<CreateAtaInterfaceResult> {
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

    let txId: TransactionSignature;

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        // c-token uses Light protocol transaction handling
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
            payer,
            blockhash,
            [],
        );
        txId = await sendAndConfirmTx(rpc, tx, confirmOptions);
    } else {
        // SPL Token / Token-2022 use standard transaction
        const transaction = new Transaction().add(ix);
        txId = await sendAndConfirmTransaction(
            rpc,
            transaction,
            [payer],
            confirmOptions,
        );
    }

    return { address: associatedToken, transactionSignature: txId };
}

/**
 * Create an associated token account idempotently for SPL Token, Token-2022, or Compressed Token.
 * Follows SPL Token createAssociatedTokenAccountIdempotent signature.
 * Defaults to c-token program.
 *
 * This is idempotent - if the account already exists, the instruction succeeds without error.
 *
 * Dispatches to the appropriate program based on `programId`:
 * - `CTOKEN_PROGRAM_ID` -> Compressed Token ATA (default, idempotent)
 * - `TOKEN_PROGRAM_ID` -> SPL Token ATA (idempotent)
 * - `TOKEN_2022_PROGRAM_ID` -> Token-2022 ATA (idempotent)
 *
 * @param rpc                      RPC connection
 * @param payer                    Fee payer and transaction signer
 * @param mint                     Mint address
 * @param owner                    Owner of the associated token account
 * @param allowOwnerOffCurve       Allow owner to be a PDA (default: false)
 * @param confirmOptions           Options for confirming the transaction
 * @param programId                Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param associatedTokenProgramId Associated token program ID (auto-derived if not provided)
 * @param ctokenConfig             Optional c-token-specific configuration
 *
 * @example
 * // Create or get existing c-token ATA (default)
 * const { address } = await createAtaInterfaceIdempotent(
 *     rpc,
 *     payer,
 *     mint,
 *     wallet.publicKey,
 * );
 */
export async function createAtaInterfaceIdempotent(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
    ctokenConfig?: CTokenConfig,
): Promise<CreateAtaInterfaceResult> {
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

    let txId: TransactionSignature;

    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        // c-token uses Light protocol transaction handling
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
            payer,
            blockhash,
            [],
        );
        txId = await sendAndConfirmTx(rpc, tx, confirmOptions);
    } else {
        // SPL Token / Token-2022 use standard transaction
        const transaction = new Transaction().add(ix);
        txId = await sendAndConfirmTransaction(
            rpc,
            transaction,
            [payer],
            confirmOptions,
        );
    }

    return { address: associatedToken, transactionSignature: txId };
}
