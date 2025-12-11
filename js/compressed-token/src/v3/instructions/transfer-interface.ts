import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
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
 * @param source        Source c-token account
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param payer         Payer for compressible extension top-up (optional)
 * @returns Transaction instruction for c-token transfer
 */
export function createCTokenTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    payer?: PublicKey,
): TransactionInstruction {
    // Instruction data format (from CTOKEN_TRANSFER.md):
    // byte 0: discriminator (3)
    // byte 1: padding (0)
    // bytes 2-9: amount (u64 LE) - SPL TokenInstruction::Transfer format
    const data = Buffer.alloc(10);
    data.writeUInt8(CTOKEN_TRANSFER_DISCRIMINATOR, 0);
    data.writeUInt8(0, 1); // padding
    data.writeBigUInt64LE(BigInt(amount), 2);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: false },
    ];

    // Add payer as 4th account if provided and different from owner
    // (for compressible extension top-up)
    if (payer && !payer.equals(owner)) {
        keys.push({ pubkey: payer, isSigner: true, isWritable: true });
    }

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
 * @param payer         Payer for compressible top-up (optional)
 * @returns instruction for c-token transfer
 */
export function createTransferInterfaceInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: (Signer | PublicKey)[] = [],
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    payer?: PublicKey,
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
            payer,
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
