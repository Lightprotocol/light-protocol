import { babyjubjubExt } from "./babyjubjubConfig";
import { ExtPointType } from "@noble/curves/abstract/edwards";

export type LookupTable = { [key: string]: string };

/**
 * @param plaintext A 32-bit bigint
 * @returns A point on the Baby Jubjub curve
 */
export function encode(plaintext: bigint): ExtPointType {
  if (plaintext >= BigInt(2 ** 32)) {
    throw new Error("The plaintext should nit be bigger than a 32-bit bigint");
  } else return babyjubjubExt.BASE.multiplyUnsafe(plaintext);
}

/**
 * @param encoded A an encoded 32-bit bigint to a Baby Jubjub curve point
 * @param precomputeSize The size of precomputed values -> 2^precomputeSize
 * @param lookupTable The offline saved 2^precomputeSize values used to break a 32-bit ECDLP.
 * @returns
 */
export function decode(
  encoded: ExtPointType,
  precomputeSize: number,
  lookupTable: LookupTable,
): bigint {
  const range = 32 - precomputeSize;
  const rangeBound = BigInt(2) ** BigInt(range);

  for (let xlo = BigInt(0); xlo < rangeBound; xlo++) {
    let loBase = babyjubjubExt.BASE.multiplyUnsafe(xlo);
    let key = encoded.subtract(loBase).toAffine().x.toString();

    if (lookupTable.hasOwnProperty(key)) {
      return xlo + rangeBound * BigInt("0x" + lookupTable[key]);
    }
  }
  throw new Error("Not Found!");
}

/**
 * @param input A 64-bit bigint
 * @returns An array of two bigints [xlo, xhi] such as `input = xlo + 2^32 * xhi`
 */
export function split64BigInt(input: bigint): [bigint, bigint] {
  /// Pad zeros to a binary string to obtain a length of 64 characters
  const padBinary = (x: string) => {
    return "0".repeat(64 - x.length) + x;
  };

  if (input >= 2 ** 64) {
    throw new Error("The input should be 64-bit bigint");
  } else {
    const bin64 = padBinary(input.toString(2));
    // the first 32 bits -> xlo
    const xhi = "0b" + bin64.substring(0, 32);
    // the last 32 bits -> xhi
    const xlo = "0b" + bin64.substring(32, 64);
    return [BigInt(xlo), BigInt(xhi)];
  }
}
