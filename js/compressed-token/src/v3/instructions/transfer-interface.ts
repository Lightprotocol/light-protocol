import {
    PublicKey,
    Signer,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferInstruction as createSplTransferInstruction,
} from '@solana/spl-token';

/**
 * c-token transfer instruction discriminator
 */
const CTOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * Create a c-token transfer instruction.
 *
 * For c-token accounts with compressible extension, the program needs
 * system_program and fee_payer to handle rent top-ups.
 *
 * @param source        Source c-token account
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param feePayer      Optional fee payer for top-ups (defaults to owner)
 * @returns Transaction instruction for c-token transfer
 */
export function createCTokenTransferInstruction(
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
    data.writeUInt8(CTOKEN_TRANSFER_DISCRIMINATOR, 0);
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
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer instruction for SPL/T22/c-token. Defaults to c-token
 * program. For cross-program transfers (SPL <> c-token), use `wrap`/`unwrap`.
 *
 * @param source        Source token account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @returns instruction for c-token transfer
 */
export function createTransferInterfaceInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: (Signer | PublicKey)[] = [],
    programId: PublicKey = CTOKEN_PROGRAM_ID,
): TransactionInstruction {
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        if (multiSigners.length > 0) {
            throw new Error(
                'c-token transfer does not support multi-signers. Use a single owner.',
            );
        }
        return createCTokenTransferInstruction(
            source,
            destination,
            owner,
            amount,
        );
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return createSplTransferInstruction(
            source,
            destination,
            owner,
            amount,
            multiSigners.map(pk =>
                pk instanceof PublicKey ? pk : pk.publicKey,
            ),
            programId,
        );
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
