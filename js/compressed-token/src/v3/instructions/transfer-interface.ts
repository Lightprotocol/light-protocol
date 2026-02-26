import {
    PublicKey,
    Signer,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferInstruction as createSplTransferInstruction,
    createTransferCheckedInstruction as createSplTransferCheckedInstruction,
} from '@solana/spl-token';

/**
 * Light token transfer instruction discriminator
 */
const LIGHT_TOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * Light token transfer_checked instruction discriminator (SPL-compatible)
 */
const LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

/**
 * Create a light-token transfer instruction.
 *
 * For light-token accounts with compressible extension, the program needs
 * system_program and fee_payer to handle rent top-ups.
 *
 * @param source        Source light-token account
 * @param destination   Destination light-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param feePayer      Optional fee payer for top-ups (defaults to owner)
 * @returns Transaction instruction for light-token transfer
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

/**
 * Create a light-token transfer_checked instruction.
 *
 * Account order matches SPL Token's transferChecked:
 * [source, mint, destination, authority]
 *
 * On-chain, the program validates that `decimals` matches the mint's decimals
 * field, preventing decimal-related transfer errors.
 *
 * @param source        Source light-token account
 * @param mint          Mint account (used for decimals validation)
 * @param destination   Destination light-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @returns Transaction instruction for light-token transfer_checked
 */
export function createLightTokenTransferCheckedInstruction(
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
    data.writeUInt8(LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: true },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer instruction for SPL/T22/light-token. Defaults to
 * light-token program.
 *
 * @param source        Source token account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param multiSigners  Multi-signers (SPL/T22 only)
 * @param programId     Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @returns instruction for transfer
 */
export function createTransferInterfaceInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: (Signer | PublicKey)[] = [],
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): TransactionInstruction {
    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        if (multiSigners.length > 0) {
            throw new Error(
                'Light token transfer does not support multi-signers. Use a single owner.',
            );
        }
        return createLightTokenTransferInstruction(
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
 * Construct a transfer_checked instruction for SPL/T22/light-token. Defaults to
 * light-token program. On-chain, validates that `decimals` matches the mint.
 *
 * @param source        Source token account
 * @param mint          Mint account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @param multiSigners  Multi-signers (SPL/T22 only)
 * @param programId     Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
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
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): TransactionInstruction {
    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        if (multiSigners.length > 0) {
            throw new Error(
                'Light token transfer does not support multi-signers. Use a single owner.',
            );
        }
        return createLightTokenTransferCheckedInstruction(
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
