import { TransactionInstruction } from '@solana/web3.js';

/** Solana maximum transaction size in bytes. */
export const MAX_TRANSACTION_SIZE = 1232;

/**
 * Conservative size budget for a combined batch (load + transfer + ATA).
 * Leaves headroom below MAX_TRANSACTION_SIZE for edge-case key counts.
 */
export const MAX_COMBINED_BATCH_BYTES = 900;

/**
 * Conservative size budget for a load-only or setup-only batch.
 */
export const MAX_LOAD_ONLY_BATCH_BYTES = 1000;

/**
 * Encode length as compact-u16 (Solana's variable-length encoding).
 * Returns the number of bytes the encoded value occupies.
 * @internal
 */
function compactU16Size(value: number): number {
    if (value < 0x80) return 1;
    if (value < 0x4000) return 2;
    return 3;
}

/**
 * Estimate the serialized byte size of a V0 VersionedTransaction built from
 * the given instructions and signer count.
 *
 * The estimate accounts for Solana's account-key deduplication: all unique
 * pubkeys across every instruction (keys + programIds) are collected into a
 * single set, matching the behaviour of
 * `TransactionMessage.compileToV0Message`.
 *
 * This intentionally does NOT use address lookup tables, so the result is an
 * upper bound. If lookup tables are used at send time the actual size will be
 * smaller.
 *
 * @param instructions  The instructions that will be included in the tx.
 * @param numSigners    Number of signers (determines signature count).
 * @returns Estimated byte size of the serialized transaction.
 */
export function estimateTransactionSize(
    instructions: TransactionInstruction[],
    numSigners: number,
): number {
    // 1. Collect unique account keys (pubkeys + programIds)
    const uniqueKeys = new Set<string>();
    for (const ix of instructions) {
        uniqueKeys.add(ix.programId.toBase58());
        for (const key of ix.keys) {
            uniqueKeys.add(key.pubkey.toBase58());
        }
    }
    const numKeys = uniqueKeys.size;

    // 2. Signatures section
    const signaturesSize = compactU16Size(numSigners) + 64 * numSigners;

    // 3. Message
    const messagePrefix = 1; // V0 prefix byte (0x80)
    const header = 3; // numRequiredSignatures, numReadonlySignedAccounts, numReadonlyUnsignedAccounts
    const accountKeysSize = compactU16Size(numKeys) + 32 * numKeys;
    const blockhashSize = 32;

    // 4. Instructions
    let instructionsSize = compactU16Size(instructions.length);
    for (const ix of instructions) {
        instructionsSize += 1; // programIdIndex (u8)
        instructionsSize += compactU16Size(ix.keys.length); // accounts array length
        instructionsSize += ix.keys.length; // account indices (u8 each)
        instructionsSize += compactU16Size(ix.data.length); // data length
        instructionsSize += ix.data.length; // data bytes
    }

    // 5. Address table lookups (empty)
    const lookupTablesSize = compactU16Size(0); // empty array

    return (
        signaturesSize +
        messagePrefix +
        header +
        accountKeysSize +
        blockhashSize +
        instructionsSize +
        lookupTablesSize
    );
}
