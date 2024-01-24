import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

/** BN with 254-bit max size */
export type BN31 = BN;

/** Create a BN instance with 254-bit max size and base58 capabilities  */
export const createBN31 = (
  number: string | number | BN | Buffer | Uint8Array | number[],
  base?: number | "hex" | "base58" | undefined,
  endian?: BN.Endianness | undefined,
): BN31 => {
  // if "base58" is passed, use bs58 to decode the string
  if (base === "base58") {
    const decoded = bs58.decode(number as string);
    return createBN31(decoded);
  }

  const bn = new BN(number, base, endian);
  return enforceSize(bn);
};

/**
 * Enforces a maximum size of 254 bits (31 bytes) for BN instances.
 * This is necessary for compatibility with zk-SNARKs, where hashes must be less than the field modulus (~2^254).
 */
function enforceSize(bn: BN) {
  if (bn.bitLength() > 254) {
    throw new Error("Value is too large. Max 254 bits or 31 bytes");
  }
  return bn;
}

/** Convert 254-bit BN to Base58 string. */
export function BN31toBase58(bn: BN31): string {
  const buffer = Buffer.from(bn.toArray("be", 31));
  return bs58.encode(buffer);
}
