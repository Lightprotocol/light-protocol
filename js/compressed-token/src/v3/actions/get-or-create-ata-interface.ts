import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
    TokenAccountNotFoundError,
    TokenInvalidAccountOwnerError,
    TokenInvalidMintError,
    TokenInvalidOwnerError,
} from '@solana/spl-token';
import type {
    Commitment,
    ConfirmOptions,
    PublicKey,
    Signer,
} from '@solana/web3.js';
import {
    sendAndConfirmTransaction,
    Transaction,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    createAssociatedTokenAccountInterfaceInstruction,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
} from '../instructions/create-ata-interface';
import {
    getAccountInterface,
    getAtaInterface,
    AccountInterface,
    TokenAccountSourceType,
} from '../get-account-interface';
import { getAtaProgramId } from '../ata-utils';
import { loadAta } from './load-ata';

/**
 * Retrieve the associated token account, or create it if it doesn't exist.
 *
 * @param rpc                       Connection to use
 * @param payer                     Payer of the transaction and initialization
 *                                  fees.
 * @param mint                      Mint associated with the account to set or
 *                                  verify.
 * @param owner                     Owner of the account. Pass Signer to
 *                                  auto-load cold (compressed) tokens, or
 *                                  PublicKey for read-only.
 * @param allowOwnerOffCurve        Allow the owner account to be a PDA (Program
 *                                  Derived Address).
 * @param commitment                Desired level of commitment for querying the
 *                                  state.
 * @param confirmOptions            Options for confirming the transaction
 * @param programId                 Token program ID (defaults to
 *                                  LIGHT_TOKEN_PROGRAM_ID)
 * @param associatedTokenProgramId  Associated token program ID (auto-derived if
 *                                  not provided)
 *
 * @returns AccountInterface with aggregated balance and source breakdown
 */
export async function getOrCreateAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey | Signer,
    allowOwnerOffCurve = false,
    commitment?: Commitment,
    confirmOptions?: ConfirmOptions,
    programId = LIGHT_TOKEN_PROGRAM_ID,
    associatedTokenProgramId = getAtaProgramId(programId),
): Promise<AccountInterface> {
    assertBetaEnabled();

    return _getOrCreateAtaInterface(
        rpc,
        payer,
        mint,
        owner,
        allowOwnerOffCurve,
        commitment,
        confirmOptions,
        programId,
        associatedTokenProgramId,
        false, // wrap=false for standard path
    );
}

/** @internal */
function isSigner(owner: PublicKey | Signer): owner is Signer {
    // Check for both publicKey and secretKey properties
    // A proper Signer (like Keypair) has secretKey as Uint8Array
    if (!('publicKey' in owner) || !('secretKey' in owner)) {
        return false;
    }
    // Verify secretKey is actually present and is a Uint8Array
    const signer = owner as Signer;
    return (
        signer.secretKey instanceof Uint8Array && signer.secretKey.length > 0
    );
}

/** @internal */
function getOwnerPublicKey(owner: PublicKey | Signer): PublicKey {
    return isSigner(owner) ? owner.publicKey : owner;
}

/**
 * @internal
 * Internal implementation with wrap parameter.
 */
export async function _getOrCreateAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey | Signer,
    allowOwnerOffCurve: boolean,
    commitment: Commitment | undefined,
    confirmOptions: ConfirmOptions | undefined,
    programId: PublicKey,
    associatedTokenProgramId: PublicKey,
    wrap: boolean,
): Promise<AccountInterface> {
    const ownerPubkey = getOwnerPublicKey(owner);
    const associatedToken = getAssociatedTokenAddressSync(
        mint,
        ownerPubkey,
        allowOwnerOffCurve,
        programId,
        associatedTokenProgramId,
    );

    // For c-token, use getAtaInterface which properly aggregates hot+cold balances
    // When wrap=true (unified path), also includes SPL/T22 balances
    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return getOrCreateCTokenAta(
            rpc,
            payer,
            mint,
            owner,
            associatedToken,
            commitment,
            confirmOptions,
            wrap,
            allowOwnerOffCurve,
        );
    }

    // For SPL/T22, use standard address-based lookup
    return getOrCreateSplAta(
        rpc,
        payer,
        mint,
        ownerPubkey,
        associatedToken,
        programId,
        associatedTokenProgramId,
        commitment,
        confirmOptions,
    );
}

/**
 * Get or create c-token ATA with proper cold balance handling.
 *
 * Like SPL's getOrCreateAssociatedTokenAccount, this is a write operation:
 * 1. Creates hot ATA if it doesn't exist
 * 2. If owner is Signer: loads cold (compressed) tokens into hot ATA
 * 3. When wrap=true and owner is Signer: also wraps SPL/T22 tokens
 *
 * After this call (with Signer owner), all tokens are in the hot ATA and ready
 * to use.
 *
 * @internal
 */
async function getOrCreateCTokenAta(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey | Signer,
    associatedToken: PublicKey,
    commitment?: Commitment,
    confirmOptions?: ConfirmOptions,
    wrap = false,
    allowOwnerOffCurve = false,
): Promise<AccountInterface> {
    const ownerPubkey = getOwnerPublicKey(owner);
    const ownerIsSigner = isSigner(owner);

    let accountInterface: AccountInterface;
    let hasHotAccount = false;

    try {
        // Use getAtaInterface which properly fetches by owner+mint and aggregates
        // hot+cold balances. When wrap=true, also includes SPL/T22 balances.
        accountInterface = await getAtaInterface(
            rpc,
            associatedToken,
            ownerPubkey,
            mint,
            commitment,
            LIGHT_TOKEN_PROGRAM_ID,
            wrap,
            allowOwnerOffCurve,
        );

        // Check if we have a hot account
        hasHotAccount =
            accountInterface._sources?.some(
                s => s.type === TokenAccountSourceType.CTokenHot,
            ) ?? false;
    } catch (error: unknown) {
        if (
            error instanceof TokenAccountNotFoundError ||
            error instanceof TokenInvalidAccountOwnerError
        ) {
            // No account found (neither hot nor cold), create hot ATA
            await createCTokenAtaIdempotent(
                rpc,
                payer,
                mint,
                ownerPubkey,
                associatedToken,
                confirmOptions,
            );

            // Fetch the newly created account
            accountInterface = await getAtaInterface(
                rpc,
                associatedToken,
                ownerPubkey,
                mint,
                commitment,
                LIGHT_TOKEN_PROGRAM_ID,
                wrap,
                allowOwnerOffCurve,
            );
            hasHotAccount = true;
        } else {
            throw error;
        }
    }

    // If we only have cold balance (no hot ATA), create the hot ATA first
    if (!hasHotAccount) {
        await createCTokenAtaIdempotent(
            rpc,
            payer,
            mint,
            ownerPubkey,
            associatedToken,
            confirmOptions,
        );
    }

    // Only auto-load if owner is a Signer (we can sign the load transaction)
    // Use direct type guard in the if condition for proper type narrowing
    if (isSigner(owner)) {
        // Check if we need to load tokens into the hot ATA
        // Load if: cold balance exists, or (wrap=true and SPL/T22 balance exists)
        const sources = accountInterface._sources ?? [];
        const hasCold = sources.some(
            s =>
                s.type === TokenAccountSourceType.CTokenCold &&
                s.amount > BigInt(0),
        );
        const hasSplToWrap =
            wrap &&
            sources.some(
                s =>
                    (s.type === TokenAccountSourceType.Spl ||
                        s.type === TokenAccountSourceType.Token2022) &&
                    s.amount > BigInt(0),
            );

        if (hasCold || hasSplToWrap) {
            // Verify owner is a valid Signer before loading
            if (
                !(owner.secretKey instanceof Uint8Array) ||
                owner.secretKey.length === 0
            ) {
                throw new Error(
                    'Owner must be a valid Signer with secretKey to auto-load',
                );
            }

            // Load all tokens into hot ATA (decompress cold, wrap SPL/T22 if
            // wrap=true)
            await loadAta(
                rpc,
                associatedToken,
                owner, // TypeScript now knows owner is Signer
                mint,
                payer,
                confirmOptions,
                undefined,
                wrap,
            );

            // Re-fetch the updated account state
            accountInterface = await getAtaInterface(
                rpc,
                associatedToken,
                ownerPubkey,
                mint,
                commitment,
                LIGHT_TOKEN_PROGRAM_ID,
                wrap,
                allowOwnerOffCurve,
            );
        }
    }

    const account = accountInterface.parsed;

    if (!account.mint.equals(mint)) throw new TokenInvalidMintError();
    if (!account.owner.equals(ownerPubkey)) throw new TokenInvalidOwnerError();

    return accountInterface;
}

/**
 * Create c-token ATA idempotently.
 * @internal
 */
async function createCTokenAtaIdempotent(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    associatedToken: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<void> {
    try {
        const ix = createAssociatedTokenAccountInterfaceIdempotentInstruction(
            payer.publicKey,
            associatedToken,
            owner,
            mint,
            LIGHT_TOKEN_PROGRAM_ID,
        );

        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
            payer,
            blockhash,
            [],
        );

        await sendAndConfirmTx(rpc, tx, confirmOptions);
    } catch {
        // Ignore errors - ATA may already exist
    }
}

/**
 * Get or create SPL/T22 ATA.
 * @internal
 */
async function getOrCreateSplAta(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    associatedToken: PublicKey,
    programId: PublicKey,
    associatedTokenProgramId: PublicKey,
    commitment?: Commitment,
    confirmOptions?: ConfirmOptions,
): Promise<AccountInterface> {
    let accountInterface: AccountInterface;

    try {
        accountInterface = await getAccountInterface(
            rpc,
            associatedToken,
            commitment,
            programId,
        );
    } catch (error: unknown) {
        // TokenAccountNotFoundError can be possible if the associated address
        // has already received some lamports, becoming a system account.
        if (
            error instanceof TokenAccountNotFoundError ||
            error instanceof TokenInvalidAccountOwnerError
        ) {
            // As this isn't atomic, it's possible others can create associated
            // accounts meanwhile.
            try {
                const transaction = new Transaction().add(
                    createAssociatedTokenAccountInterfaceInstruction(
                        payer.publicKey,
                        associatedToken,
                        owner,
                        mint,
                        programId,
                        associatedTokenProgramId,
                    ),
                );

                await sendAndConfirmTransaction(
                    rpc,
                    transaction,
                    [payer],
                    confirmOptions,
                );
            } catch {
                // Ignore all errors; for now there is no API-compatible way to
                // selectively ignore the expected instruction error if the
                // associated account exists already.
            }

            // Now this should always succeed
            accountInterface = await getAccountInterface(
                rpc,
                associatedToken,
                commitment,
                programId,
            );
        } else {
            throw error;
        }
    }

    const account = accountInterface.parsed;

    if (!account.mint.equals(mint)) throw new TokenInvalidMintError();
    if (!account.owner.equals(owner)) throw new TokenInvalidOwnerError();

    return accountInterface;
}
