import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

/**
 * Discriminator for CTokenFreezeAccount (native instruction 10).
 * Matches InstructionType::CTokenFreezeAccount in the on-chain program.
 */
const CTOKEN_FREEZE_ACCOUNT_DISCRIMINATOR = Buffer.from([10]);

/**
 * Discriminator for CTokenThawAccount (native instruction 11).
 * Matches InstructionType::CTokenThawAccount in the on-chain program.
 */
const CTOKEN_THAW_ACCOUNT_DISCRIMINATOR = Buffer.from([11]);

/**
 * Create an instruction to freeze a decompressed c-token account.
 *
 * Freezing sets the account state to AccountState::Frozen, preventing
 * transfers and other token operations. Only the mint's freeze_authority
 * can freeze accounts.
 *
 * Account order per program:
 * 0. token_account (mutable) - the c-token account to freeze
 * 1. mint (readonly)         - the mint associated with the token account
 * 2. freeze_authority        - must match mint.freeze_authority (signer)
 *
 * @param tokenAccount   The c-token account to freeze (must be Initialized)
 * @param mint           The mint of the c-token account
 * @param freezeAuthority The freeze authority of the mint (signer)
 * @returns TransactionInstruction
 */
export function createCTokenFreezeAccountInstruction(
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: PublicKey,
): TransactionInstruction {
    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: tokenAccount, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: freezeAuthority, isSigner: true, isWritable: false },
        ],
        data: CTOKEN_FREEZE_ACCOUNT_DISCRIMINATOR,
    });
}

/**
 * Create an instruction to thaw (unfreeze) a frozen c-token account.
 *
 * Thawing restores the account state from AccountState::Frozen to
 * AccountState::Initialized, re-enabling token operations. Only the
 * mint's freeze_authority can thaw accounts.
 *
 * Account order per program:
 * 0. token_account (mutable) - the frozen c-token account to thaw
 * 1. mint (readonly)         - the mint associated with the token account
 * 2. freeze_authority        - must match mint.freeze_authority (signer)
 *
 * @param tokenAccount   The frozen c-token account to thaw
 * @param mint           The mint of the c-token account
 * @param freezeAuthority The freeze authority of the mint (signer)
 * @returns TransactionInstruction
 */
export function createCTokenThawAccountInstruction(
    tokenAccount: PublicKey,
    mint: PublicKey,
    freezeAuthority: PublicKey,
): TransactionInstruction {
    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            { pubkey: tokenAccount, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: freezeAuthority, isSigner: true, isWritable: false },
        ],
        data: CTOKEN_THAW_ACCOUNT_DISCRIMINATOR,
    });
}
