import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';

/**
 * Parameters for creating a MintTo instruction.
 */
export interface CreateMintToInstructionParams {
    /** Mint account (CMint - decompressed compressed mint) */
    mint: PublicKey;
    /** Destination CToken account to mint to */
    destination: PublicKey;
    /** Amount of tokens to mint */
    amount: number | bigint;
    /** Mint authority (must be signer) */
    authority: PublicKey;
    /** Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit) */
    maxTopUp?: number;
    /** Optional fee payer for rent top-ups. If not provided, authority pays. */
    feePayer?: PublicKey;
}

/**
 * Create instruction for minting tokens to a CToken account.
 *
 * This is a simple 3-4 account instruction for minting to decompressed CToken accounts.
 * Uses discriminator 7 (CTokenMintTo).
 *
 * @param params - Mint instruction parameters
 * @returns TransactionInstruction for minting tokens
 */
export function createMintToInstruction(
    params: CreateMintToInstructionParams,
): TransactionInstruction {
    const { mint, destination, amount, authority, maxTopUp, feePayer } = params;

    // Authority is writable only when maxTopUp is set AND no feePayer
    // (authority pays for top-ups only if no separate feePayer)
    const authorityWritable = maxTopUp !== undefined && !feePayer;

    const keys = [
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: authority, isSigner: true, isWritable: authorityWritable },
        // System program required for rent top-up CPIs
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];

    // Add fee_payer if provided (must be signer and writable)
    if (feePayer) {
        keys.push({ pubkey: feePayer, isSigner: true, isWritable: true });
    }

    // Build instruction data: discriminator (7) + amount (u64) + optional max_top_up (u16)
    const amountBigInt = BigInt(amount.toString());
    const dataSize = maxTopUp !== undefined ? 11 : 9; // 1 + 8 + optional 2
    const data = Buffer.alloc(dataSize);

    data.writeUInt8(7, 0); // CTokenMintTo discriminator
    data.writeBigUInt64LE(amountBigInt, 1);

    if (maxTopUp !== undefined) {
        data.writeUInt16LE(maxTopUp, 9);
    }

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}
