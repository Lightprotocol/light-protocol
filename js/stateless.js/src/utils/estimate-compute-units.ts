/**
 * Deterministic compute unit estimator for transactions that may include a
 * decompress step. This avoids extra simulation/RPC roundtrips by sizing the
 * compute budget offline using calibrated constants.
 */
const DEFAULT_DECOMPRESS_BASE_CU = 400_000;
const DEFAULT_DECOMPRESS_PROOF_CU = 100_000;

import {
    TransactionInstruction,
    Transaction,
    VersionedTransaction,
} from '@solana/web3.js';

export type EstimateDecompressCuOptions = {
    /** Override base decompress cost (default: 400k) */
    baseCu?: number;
    /** Override proof decompress cost (default: 100k) */
    proofCu?: number;
};

/**
 * Check if an instruction array contains a decompressAccountsIdempotent
 * instruction. Uses the instruction discriminator to identify the instruction
 * type.
 */
export function hasDecompressInstruction(
    instructions: TransactionInstruction[],
): boolean {
    // sha256("global:decompress_accounts_idempotent")
    const DECOMPRESS_DISCRIMINATOR = Buffer.from([
        0x4d, 0x9e, 0x1a, 0x7c, 0x8f, 0x2b, 0x3e, 0x5a,
    ]);

    return instructions.some(ix => {
        if (ix.data.length < 8) return false;
        const ixDiscriminator = ix.data.subarray(0, 8);
        return ixDiscriminator.equals(DECOMPRESS_DISCRIMINATOR);
    });
}

/**
 * Extract whether a decompressAccountsIdempotent instruction has a proof.
 * Instruction data format:
 * - bytes 0-7: discriminator
 * - byte 8: Option<CompressedProof> discriminator (0 = None, 1 = Some)
 * - bytes 9+: proof data (if Some) + remaining instruction data
 *
 * @returns true if proof is Some, false if None or not a decompress instruction
 */
function hasProofInDecompressInstruction(ix: TransactionInstruction): boolean {
    const DECOMPRESS_DISCRIMINATOR = Buffer.from([
        0x4d, 0x9e, 0x1a, 0x7c, 0x8f, 0x2b, 0x3e, 0x5a,
    ]);

    // Check if it's a decompress instruction
    if (ix.data.length < 9) return false;
    const ixDiscriminator = ix.data.subarray(0, 8);
    if (!ixDiscriminator.equals(DECOMPRESS_DISCRIMINATOR)) return false;

    // Byte 8 is the Option discriminator: 0 = None, 1 = Some
    return ix.data[8] === 1;
}

/**
 * Check if a Transaction or VersionedTransaction contains a decompressAccountsIdempotent instruction.
 * Wrapper around hasDecompressInstruction that extracts instructions from transaction objects.
 */
export function hasDecompressInTransaction(
    transaction: Transaction | VersionedTransaction,
): boolean {
    if (transaction instanceof VersionedTransaction) {
        const message = transaction.message;
        const instructions: TransactionInstruction[] =
            message.compiledInstructions.map(compiledIx => ({
                programId: message.staticAccountKeys[compiledIx.programIdIndex],
                keys: compiledIx.accountKeyIndexes.map(keyIndex => ({
                    pubkey:
                        message.staticAccountKeys[keyIndex] ||
                        message.addressTableLookups?.[0]?.readonlyIndexes?.[
                            keyIndex - message.staticAccountKeys.length
                        ],
                    isSigner: keyIndex < message.header.numRequiredSignatures,
                    isWritable:
                        keyIndex <
                            message.header.numRequiredSignatures -
                                message.header.numReadonlySignedAccounts ||
                        (keyIndex >= message.header.numRequiredSignatures &&
                            keyIndex <
                                message.staticAccountKeys.length -
                                    message.header.numReadonlyUnsignedAccounts),
                })),
                data: Buffer.from(compiledIx.data),
            }));
        return hasDecompressInstruction(instructions);
    } else {
        return hasDecompressInstruction(transaction.instructions);
    }
}

/**
 * Estimate compute units needed for decompression in an instruction array.
 * Formula: base_cu + (proof_cu if proof is Some)
 * Default: 400k base + 100k if proof present
 *
 * @param instructions - Array of transaction instructions to analyze
 * @param opts - Optional overrides for base and proof CU costs
 * @returns CU needed for decompression (0 if no decompress instruction)
 */
export function estimateDecompressCu(
    instructions: TransactionInstruction[],
    opts?: EstimateDecompressCuOptions,
): number {
    const baseCu = opts?.baseCu ?? DEFAULT_DECOMPRESS_BASE_CU;
    const proofCu = opts?.proofCu ?? DEFAULT_DECOMPRESS_PROOF_CU;

    // Check if there's any decompress instruction
    const hasDecompress = hasDecompressInstruction(instructions);
    if (!hasDecompress) return 0;

    // Check if any decompress instruction has a proof
    const hasProof = instructions.some(hasProofInDecompressInstruction);

    return baseCu + (hasProof ? proofCu : 0);
}

/**
 * Estimate compute units needed for decompression in a transaction.
 * Formula: base_cu + (proof_cu if proof is Some)
 * Default: 400k base + 100k if proof present
 *
 * @param transaction - Transaction or VersionedTransaction to analyze
 * @param opts - Optional overrides for base and proof CU costs
 * @returns CU needed for decompression (0 if no decompress instruction)
 */
export function estimateDecompressCuForTransaction(
    transaction: Transaction | VersionedTransaction,
    opts?: EstimateDecompressCuOptions,
): number {
    if (transaction instanceof VersionedTransaction) {
        const message = transaction.message;
        const instructions: TransactionInstruction[] =
            message.compiledInstructions.map(compiledIx => ({
                programId: message.staticAccountKeys[compiledIx.programIdIndex],
                keys: compiledIx.accountKeyIndexes.map(keyIndex => ({
                    pubkey:
                        message.staticAccountKeys[keyIndex] ||
                        message.addressTableLookups?.[0]?.readonlyIndexes?.[
                            keyIndex - message.staticAccountKeys.length
                        ],
                    isSigner: keyIndex < message.header.numRequiredSignatures,
                    isWritable:
                        keyIndex <
                            message.header.numRequiredSignatures -
                                message.header.numReadonlySignedAccounts ||
                        (keyIndex >= message.header.numRequiredSignatures &&
                            keyIndex <
                                message.staticAccountKeys.length -
                                    message.header.numReadonlyUnsignedAccounts),
                })),
                data: Buffer.from(compiledIx.data),
            }));
        return estimateDecompressCu(instructions, opts);
    } else {
        return estimateDecompressCu(transaction.instructions, opts);
    }
}
