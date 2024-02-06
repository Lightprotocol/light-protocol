import {BN254, createBN254} from "../utxo/bn254";
import {sha256} from "@noble/hashes/sha256";
import {PublicKey, SystemProgram} from "@solana/web3.js";

/**
 * Truncates the given 32-byte array to a 31-byte one, ensuring it fits
 * into the Fr modulo field.
 *
 * ## Safety
 *
 * This function is primarily used for truncating hashes (e.g., SHA-256) which are
 * not constrained by any modulo space. It's important to note that, as of now,
 * it's not possible to use any ZK-friendly function within a single transaction.
 * While truncating hashes to 31 bytes is generally safe, you should ensure that
 * this operation is appropriate for your specific use case.
 *
 * @param bytes The 32-byte array to be truncated.
 * @returns The truncated 31-byte array.
 *
 * @example
 * ```typescript
 * // example usage of truncate function
 * const truncated = truncateFunction(original32BytesArray);
 * ```
 */
export function truncateToCircuit(digest: Uint8Array): BN254 {
    return createBN254(digest.slice(1, 32), undefined, "be");
}

export function hashAndTruncateToCircuit(data: Uint8Array): BN254 {
    return truncateToCircuit(sha256.create().update(Buffer.from(data)).digest());
}

/**
 * Hashes and truncates assets to fit within 254-bit modulo space.
 * Returns decimal string.
 * SOL = "0"
 * */
export function stringifyAssetsToCircuitInput(assets: PublicKey[]): string[] {
    return assets.map((asset: PublicKey, index) => {
        if (index !== 0 && asset.toBase58() === SystemProgram.programId.toBase58())
            return "0";
        return hashAndTruncateToCircuit(asset.toBytes()).toString();
    });
}
