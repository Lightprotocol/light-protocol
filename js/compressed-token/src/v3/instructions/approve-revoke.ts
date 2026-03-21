import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

const LIGHT_TOKEN_APPROVE_DISCRIMINATOR = 4;
const LIGHT_TOKEN_REVOKE_DISCRIMINATOR = 5;

/**
 * Create an instruction to approve a delegate for a light-token account.
 *
 * Account order per program:
 * 0. token_account (mutable)   - the light-token account
 * 1. delegate      (readonly)  - the delegate to approve
 * 2. owner         (signer)    - owner of the token account
 * 3. system_program (readonly) - for rent top-ups via CPI
 * 4. fee_payer     (mutable, signer) - pays for rent top-ups
 *
 * @param tokenAccount The light-token account to set delegation on
 * @param delegate     The delegate to approve
 * @param owner        Owner of the token account (signer)
 * @param amount       Amount of tokens to delegate
 * @param feePayer     Optional fee payer for rent top-ups (defaults to owner)
 * @returns TransactionInstruction
 */
export function createLightTokenApproveInstruction(
    tokenAccount: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    feePayer?: PublicKey,
): TransactionInstruction {
    const data = Buffer.alloc(9);
    data.writeUInt8(LIGHT_TOKEN_APPROVE_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const effectiveFeePayer = feePayer ?? owner;

    const keys = [
        { pubkey: tokenAccount, isSigner: false, isWritable: true },
        { pubkey: delegate, isSigner: false, isWritable: false },
        {
            pubkey: owner,
            isSigner: true,
            isWritable: effectiveFeePayer.equals(owner),
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: effectiveFeePayer,
            isSigner: !effectiveFeePayer.equals(owner),
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Create an instruction to revoke delegation for a light-token account.
 *
 * Account order per program:
 * 0. token_account (mutable)   - the light-token account
 * 1. owner         (signer)    - owner of the token account
 * 2. system_program (readonly) - for rent top-ups via CPI
 * 3. fee_payer     (mutable, signer) - pays for rent top-ups
 *
 * @param tokenAccount The light-token account to revoke delegation on
 * @param owner        Owner of the token account (signer)
 * @param feePayer     Optional fee payer for rent top-ups (defaults to owner)
 * @returns TransactionInstruction
 */
export function createLightTokenRevokeInstruction(
    tokenAccount: PublicKey,
    owner: PublicKey,
    feePayer?: PublicKey,
): TransactionInstruction {
    const effectiveFeePayer = feePayer ?? owner;

    const keys = [
        { pubkey: tokenAccount, isSigner: false, isWritable: true },
        {
            pubkey: owner,
            isSigner: true,
            isWritable: effectiveFeePayer.equals(owner),
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: effectiveFeePayer,
            isSigner: !effectiveFeePayer.equals(owner),
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data: Buffer.from([LIGHT_TOKEN_REVOKE_DISCRIMINATOR]),
    });
}
