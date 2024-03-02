// import { Buffer } from "buffer";
export const toArray = <T>(value: T | T[]) =>
  Array.isArray(value) ? value : [value];

export function bigintToArray(bi: bigint): number[] {
  // Assuming bi is a positive bigint
  let hex = bi.toString(16);
  if (hex.length % 2) {
    hex = "0" + hex;
  } // Ensure even length
  const len = hex.length / 2;
  const u8 = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    u8[i] = parseInt(hex.substring(i * 2, 2), 16);
  }
  return Array.from(u8);
}

export function arrayToBigint(byteArray: number[]): bigint {
  let result = BigInt(0);
  for (let i = 0; i < byteArray.length; i++) {
    let exponent = BigInt(byteArray.length - i - 1);
    let base = BigInt(256);
    let power = BigInt(1);
    while (exponent > 0) {
      power *= base;
      exponent--;
    }
    result += BigInt(byteArray[i]) * power;
  }
  return result;
}

export const bufToDecStr = (buf: Buffer): string => {
  return BigInt(`0x${buf.toString("hex")}`).toString();
};

function isSmallerThanBn254FieldSizeLe(bytes: Buffer): boolean {
  const bn254Modulus = BigInt(
    "0x6500000000000000000000000000000000000000000000000000000000000000"
  );
  const bigint = BigInt("0x" + bytes.toString("hex"));
  return bigint < bn254Modulus;
}

export async function hashToBn254FieldSizeLe(
  bytes: Buffer
): Promise<[Buffer, number] | null> {
  let bumpSeed = 255; // Start with the max value for a byte
  while (bumpSeed >= 0) {
    let hashedValue;
    if (typeof crypto.subtle !== "undefined") {
      // Browser
      hashedValue = await crypto.subtle.digest("SHA-256", bytes);
    } else {
      // Node.js
      const hash = require("crypto").createHash("sha256");
      hash.update(bytes);
      hashedValue = hash.digest();
    }

    // Truncate to 31 bytes so that value is less than bn254 Fr modulo field size
    hashedValue[0] = 0;
    hashedValue[1] = 0;

    if (isSmallerThanBn254FieldSizeLe(hashedValue)) {
      return [hashedValue, bumpSeed];
    }

    bumpSeed -= 1;
  }
  return null;
}

/** Mutates input array in place */
export function pushUniqueItems<T>(items: T[], map: T[]): void {
  items.forEach((item) => {
    if (!map.includes(item)) {
      map.push(item);
    }
  });
}
