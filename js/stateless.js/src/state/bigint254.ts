import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { FIELD_SIZE } from "../constants";

/** bigint with <254-bit max size */
export type bigint254 = bigint;

/** Create a bigint instance with <254-bit max size and base58 capabilities */
export const createBigint254 = (
  number: string | number | bigint | Buffer | Uint8Array | number[],
  base?: number | "hex" | "base58" | undefined
): bigint254 => {
  if (base === "base58") {
    if (typeof number !== "string") throw new Error("Must be a base58 string");
    return createBigint254(Buffer.from(bs58.decode(number)));
  }

  const bigintNumber = convertToBigInt(number, base);

  return enforceSize(bigintNumber);
};

const convertToBigInt = (
  number: string | number | bigint | Buffer | Uint8Array | number[],
  base?: number | "hex" | "base58" | undefined
): bigint => {
  if (typeof number === "string")
    return base === "hex" ? BigInt("0x" + number) : BigInt(number);
  else if (typeof number === "number")
    return BigInt("0x" + Buffer.from([number]).toString("hex"));
  else if (Array.isArray(number) || ArrayBuffer.isView(number))
    return BigInt("0x" + Buffer.from(number).toString("hex"));
  else return BigInt(number);
};

/**
 * Enforces a maximum size of <254 bits for bigint instances.
 * This is necessary for compatibility with zk-SNARKs, where hashes must be less than the field modulus (~2^254).
 */
function enforceSize(bigintNumber: bigint): bigint254 {
  if (bigintNumber >= FIELD_SIZE) {
    throw new Error("Value is too large. Max <254 bits");
  }
  return bigintNumber;
}

/** Convert <254-bit bigint to Base58 string. Fills up to 32 bytes. */
export function bigint254toBase58(bigintNumber: bigint254): string {
  const buffer = Buffer.from(bigintNumber.toString(16), "hex");
  return bs58.encode(buffer);
}
