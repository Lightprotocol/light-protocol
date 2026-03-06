import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

/**
 * Discriminator for LightTokenFreezeAccount (native instruction 10).
 * Matches InstructionType::LightTokenFreezeAccount in the on-chain program.
 */
const LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR = Buffer.from([10]);

/**
 * Discriminator for LightTokenThawAccount (native instruction 11).
 * Matches InstructionType::LightTokenThawAccount in the on-chain program.
 */
const LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR = Buffer.from([11]);

/**
 * Create an instruction to freeze a decompressed light-token account.
 *
 * Freezing sets the account state to AccountState::Frozen, preventing
 * transfers and other token operations. Only the mint's freeze_authority
 * can freeze accounts.
 *
 * Account order per program:
 * 0. token_account (mutable) - the light-token account to freeze
 * 1. mint (readonly)         - the mint associated with the token account
 * 2. freeze_authority        - must match mint.freeze_authority (signer)
 *
 * @param tokenAccount   The light-token account to freeze (must be Initialized)
 * @param mint           The mint of the light-token account
 * @param freezeAuthority The freeze authority of the mint (signer)
 * @returns TransactionInstruction
 */
export function createLightTokenFreezeAccountInstruction(
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
        data: LIGHT_TOKEN_FREEZE_ACCOUNT_DISCRIMINATOR,
    });
}

/**
 * Create an instruction to thaw (unfreeze) a frozen light-token account.
 *
 * Thawing restores the account state from AccountState::Frozen to
 * AccountState::Initialized, re-enabling token operations. Only the
 * mint's freeze_authority can thaw accounts.
 *
 * Account order per program:
 * 0. token_account (mutable) - the frozen light-token account to thaw
 * 1. mint (readonly)         - the mint associated with the token account
 * 2. freeze_authority        - must match mint.freeze_authority (signer)
 *
 * @param tokenAccount   The frozen light-token account to thaw
 * @param mint           The mint of the light-token account
 * @param freezeAuthority The freeze authority of the mint (signer)
 * @returns TransactionInstruction
 */
export function createLightTokenThawAccountInstruction(
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
        data: LIGHT_TOKEN_THAW_ACCOUNT_DISCRIMINATOR,
    });
}
