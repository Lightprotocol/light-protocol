import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

/**
 * Light token transfer instruction discriminator
 */
const LIGHT_TOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * Create a Light token transfer instruction.
 *
 * For c-token accounts with compressible extension, the program needs
 * system_program and fee_payer to handle rent top-ups.
 *
 * @param source        Source c-token account
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param feePayer      Optional fee payer for top-ups (defaults to owner)
 * @returns Transaction instruction for Light token transfer
 */
export function createLightTokenTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    feePayer?: PublicKey,
): TransactionInstruction {
    // Instruction data format:
    // byte 0: discriminator (3)
    // bytes 1-8: amount (u64 LE)
    const data = Buffer.alloc(9);
    data.writeUInt8(LIGHT_TOKEN_TRANSFER_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const effectiveFeePayer = feePayer ?? owner;

    // Account order per program:
    // 0: source (writable)
    // 1: destination (writable)
    // 2: authority/owner (signer, writable for top-ups)
    // 3: system_program (for top-ups via CPI)
    // 4: fee_payer (signer, writable - pays for top-ups)
    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: effectiveFeePayer,
            isSigner: !effectiveFeePayer.equals(owner), // Only mark as signer if different from owner (owner already signed)
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}
