import { CTOKEN_PROGRAM_ID, Rpc } from '@lightprotocol/stateless.js';
import {
    Account,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    getAccount,
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
    Connection,
    PublicKey,
    Signer,
} from '@solana/web3.js';
import { sendAndConfirmTransaction, Transaction } from '@solana/web3.js';
import { getAccountInterface } from '../get-account-interface';
import { createAssociatedCTokenAccountInstruction } from '../instructions/create-associated-ctoken';
// import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '../constants.js';

// import { createAssociatedTokenAccountInstruction } from '../instructions/associatedTokenAccount.js';
// import type { Account } from '../state/account.js';
// import { getAccount } from '../state/account.js';
// import { getAssociatedTokenAddressSync } from '../state/mint.js';

/**
 * Retrieve the associated token account, or create it if it doesn't exist
 *
 * @param rpc                      Connection to use
 * @param payer                    Payer of the transaction and initialization fees
 * @param mint                     Mint associated with the account to set or verify
 * @param owner                    Owner of the account to set or verify
 * @param allowOwnerOffCurve       Allow the owner account to be a PDA (Program Derived Address)
 * @param commitment               Desired level of commitment for querying the state
 * @param confirmOptions           Options for confirming the transaction
 * @param programId                SPL Token program account or C token program account
 * @param associatedTokenProgramId SPL Associated Token program account or C token program account
 *
 * @return Address of the new associated token account
 */
export async function getOrCreateAssociatedTokenAccountInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    commitment?: Commitment,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = programId === CTOKEN_PROGRAM_ID
        ? CTOKEN_PROGRAM_ID
        : ASSOCIATED_TOKEN_PROGRAM_ID,
): Promise<Account> {
    const associatedToken = getAssociatedTokenAddressSync(
        mint,
        owner,
        allowOwnerOffCurve,
        programId,
        associatedTokenProgramId,
    );

    // This is the optimal logic, considering TX fee, client-side computation, RPC roundtrips and guaranteed idempotent.
    // Sadly we can't do this atomically.
    let account: Account;
    try {
        const accountInterface = await getAccountInterface(
            rpc,
            associatedToken,
            commitment,
            programId,
        );
        account = accountInterface.parsed;
    } catch (error: unknown) {
        // TokenAccountNotFoundError can be possible if the associated address has already received some lamports,
        // becoming a system account. Assuming program derived addressing is safe, this is the only case for the
        // TokenInvalidAccountOwnerError in this code path.
        if (
            error instanceof TokenAccountNotFoundError ||
            error instanceof TokenInvalidAccountOwnerError
        ) {
            // As this isn't atomic, it's possible others can create associated accounts meanwhile.
            try {
                const transaction = new Transaction().add(
                    createAssociatedCTokenAccountInstruction(
                        payer.publicKey,
                        // associatedToken,
                        owner,
                        mint,
                        // programId,
                        // associatedTokenProgramId,
                    ),
                );

                await sendAndConfirmTransaction(
                    rpc,
                    transaction,
                    [payer],
                    confirmOptions,
                );
            } catch (error: unknown) {
                // Ignore all errors; for now there is no API-compatible way to selectively ignore the expected
                // instruction error if the associated account exists already.
            }

            // Now this should always succeed
            console.log(
                '001 associatedToken, should fetch now ',
                associatedToken.toBase58(),
            );
            const { parsed } = await getAccountInterface(
                rpc,
                associatedToken,
                commitment,
                programId,
            );
            account = parsed;
            console.log('001 account, ', account);
        } else {
            throw error;
        }
    }

    if (!account.mint.equals(mint)) throw new TokenInvalidMintError();
    if (!account.owner.equals(owner)) throw new TokenInvalidOwnerError();

    return account;
}
