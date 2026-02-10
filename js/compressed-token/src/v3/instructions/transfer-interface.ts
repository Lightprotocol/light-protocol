import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferInstruction as createSplTransferInstruction,
    createTransferCheckedInstruction as createSplTransferCheckedInstruction,
} from '@solana/spl-token';

/**
 * c-token transfer instruction discriminator
 */
const CTOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * c-token transfer_checked instruction discriminator (SPL-compatible)
 */
const CTOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

/**
 * Create a c-token transfer instruction.
 *
 * @param source        Source c-token account
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @returns Transaction instruction for c-token transfer
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

/**
 * Create a c-token transfer_checked instruction.
 *
 * Account order matches SPL Token's transferChecked:
 * [source, mint, destination, authority]
 *
 * On-chain, the program validates that `decimals` matches the mint's decimals
 * field, preventing decimal-related transfer errors.
 *
 * @param source        Source c-token account
 * @param mint          Mint account (used for decimals validation)
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @returns Transaction instruction for c-token transfer_checked
 */
export function createCTokenTransferCheckedInstruction(
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    decimals: number,
): TransactionInstruction {
    // Instruction data format:
    // byte 0: discriminator (12)
    // bytes 1-8: amount (u64 LE)
    // byte 9: decimals (u8)
    const data = Buffer.alloc(10);
    data.writeUInt8(CTOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: true },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer_checked instruction for SPL/T22/c-token. Defaults to
 * c-token program. On-chain, validates that `decimals` matches the mint.
 *
 * @param source        Source token account
 * @param mint          Mint account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @param multiSigners  Multi-signers (SPL/T22 only)
 * @param programId     Token program ID (default: CTOKEN_PROGRAM_ID)
 * @returns instruction for transfer_checked
 */
export function createTransferInterfaceCheckedInstruction(
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    decimals: number,
    multiSigners: (Signer | PublicKey)[] = [],
    programId: PublicKey = CTOKEN_PROGRAM_ID,
): TransactionInstruction {
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        if (multiSigners.length > 0) {
            throw new Error(
                'c-token transfer does not support multi-signers. Use a single owner.',
            );
        }
        return createCTokenTransferCheckedInstruction(
            source,
            mint,
            destination,
            owner,
            amount,
            decimals,
        );
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return createSplTransferCheckedInstruction(
            source,
            mint,
            destination,
            owner,
            amount,
            decimals,
            multiSigners.map(pk =>
                pk instanceof PublicKey ? pk : pk.publicKey,
            ),
            programId,
        );
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
