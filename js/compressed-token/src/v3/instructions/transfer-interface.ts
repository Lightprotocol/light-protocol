import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferInstruction as createSplTransferInstruction,
} from '@solana/spl-token';

/**
 * light-token transfer instruction discriminator
 */
const CTOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * Create a light-token transfer instruction.
 *
 * @param source        Source light-token account
 * @param destination   Destination light-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @returns Transaction instruction for light-token transfer
 */
export function createCTokenTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
): TransactionInstruction {
    // Instruction data format:
    // byte 0: discriminator (3)
    // bytes 1-8: amount (u64 LE)
    const data = Buffer.alloc(9);
    data.writeUInt8(CTOKEN_TRANSFER_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: true }, // owner is also payer for top-ups
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer instruction for SPL/T22/light-token. Defaults to light-token
 * program. For cross-program transfers (SPL <> light-token), use `wrap`/`unwrap`.
 *
 * @param source        Source token account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @returns instruction for light-token transfer
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
