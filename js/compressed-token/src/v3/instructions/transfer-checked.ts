import {
    PublicKey,
    Signer,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferCheckedInstruction as createSplTransferCheckedInstruction,
} from '@solana/spl-token';

/**
 * c-token transfer_checked instruction discriminator
 */
const CTOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

/**
 * Create a c-token transfer_checked instruction.
 *
 * Validates the transfer amount against the mint's decimals on-chain.
 *
 * @param source        Source c-token account
 * @param mint          Mint account (read-only, used for decimals validation)
 * @param destination   Destination c-token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @param feePayer      Optional separate fee payer for top-ups
 * @returns Transaction instruction for c-token transfer_checked
 */
export function createCTokenTransferCheckedInstruction(
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    decimals: number,
    feePayer?: PublicKey,
): TransactionInstruction {
    // Instruction data format:
    // byte 0: discriminator (12)
    // bytes 1-8: amount (u64 LE)
    // byte 9: decimals (u8)
    const data = Buffer.alloc(10);
    data.writeUInt8(CTOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    // Authority is writable when no feePayer (owner pays top-ups),
    // readonly when feePayer is provided (feePayer pays instead).
    const authorityIsWritable = !feePayer;

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: owner,
            isSigner: true,
            isWritable: authorityIsWritable,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ];

    if (feePayer) {
        keys.push({
            pubkey: feePayer,
            isSigner: true,
            isWritable: true,
        });
    }

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer_checked instruction for SPL/T22/c-token. Defaults to
 * c-token program. Validates amount against mint decimals.
 *
 * @param source        Source token account
 * @param mint          Mint account
 * @param destination   Destination token account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param decimals      Expected decimals of the mint
 * @param multiSigners  Multi-signers (SPL only, not supported for c-token)
 * @param programId     Token program ID (default: CTOKEN_PROGRAM_ID)
 * @returns instruction for transfer_checked
 */
export function createTransferCheckedInterfaceInstruction(
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
                'c-token transfer_checked does not support multi-signers. Use a single owner.',
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
