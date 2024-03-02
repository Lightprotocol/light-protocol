import { Buffer } from "buffer";
import crypto from "crypto";

export const toArray = <T>(value: T | T[]) =>
  Array.isArray(value) ? value : [value];

export function bigintToArray(bi: bigint): number[] {
  // Assuming bi is a positive bigint
  let hex = bi.toString(16);
  if (hex.length % 2) {
    hex = "0" + hex; // Ensure even length
  }
  const len = hex.length / 2;
  const u8 = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    // Correctly extract two characters at a time and parse
    u8[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
  }
  return Array.from(u8);
}

export function arrayToBigint(byteArray: number[]): bigint {
  let result = BigInt(0);
  for (let i = 0; i < byteArray.length; i++) {
    result = (result << BigInt(8)) + BigInt(byteArray[i]);
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

function truncateToBn254FieldSize(hashedValue: Buffer): Buffer {
  const bn254Modulus = BigInt(
    "0x6500000000000000000000000000000000000000000000000000000000000000"
  );

  let valueBigInt = BigInt("0x" + hashedValue.toString("hex"));

  /// ensure the value is less than the bn254 field size
  valueBigInt = valueBigInt % bn254Modulus;

  let truncatedValueHex = valueBigInt.toString(16);

  if (truncatedValueHex.length % 2 !== 0) {
    truncatedValueHex = "0" + truncatedValueHex;
  }

  let truncatedValueBuffer = Buffer.from(truncatedValueHex, "hex");

  if (truncatedValueBuffer.length < 32) {
    const padding = Buffer.alloc(32 - truncatedValueBuffer.length, 0);
    truncatedValueBuffer = Buffer.concat([padding, truncatedValueBuffer], 32);
  }

  return truncatedValueBuffer;
}

export async function hashToBn254FieldSizeLe(
  bytes: Buffer
): Promise<[Buffer, number] | null> {
  /// FIXME: why are we assuming the need for a bump seed?
  let bumpSeed = 255; // Start with the max value for a byte
  while (bumpSeed >= 0) {
    let hashedValue: Buffer;
    if (typeof crypto.subtle !== "undefined") {
      // Browser
      hashedValue = Buffer.from(await crypto.subtle.digest("SHA-256", bytes));
    } else {
      // Node.js
      const hash = crypto.createHash("sha256");
      hash.update(bytes);
      hashedValue = hash.digest();
    }

    // Truncate to 31 bytes so that value is less than bn254 Fr modulo field size
    hashedValue[0] = 0;
    hashedValue[1] = 0;
    // truncateToBn254FieldSize(hashedValue); // TODO: less blunt truncation

    if (isSmallerThanBn254FieldSizeLe(hashedValue)) {
      return [hashedValue, bumpSeed];
    }

    bumpSeed -= 1;
  }
  return null;
}

/** Mutates array in place */
export function pushUniqueItems<T>(items: T[], map: T[]): void {
  items.forEach((item) => {
    if (!map.includes(item)) {
      map.push(item);
    }
  });
}

// FIXME: check bundling and how to resolve the type error
//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { it, expect, describe } = import.meta.vitest;

  describe("toArray function", () => {
    it("should convert a single item to an array", () => {
      expect(toArray(1)).toEqual([1]);
    });

    it("should leave an array unchanged", () => {
      expect(toArray([1, 2, 3])).toEqual([1, 2, 3]);
    });
  });

  describe("isSmallerThanBn254FieldSizeLe function", () => {
    it("should return true for a small number", () => {
      const buf = Buffer.from(
        "0000000000000000000000000000000000000000000000000000000000000000",
        "hex"
      );
      expect(isSmallerThanBn254FieldSizeLe(buf)).toBe(true);
    });

    it("should return false for a large number", () => {
      const buf = Buffer.from(
        "6500000000000000000000000000000000000000000000000000000000000000",
        "hex"
      );
      expect(isSmallerThanBn254FieldSizeLe(buf)).toBe(false);
    });
  });

  describe("hashToBn254FieldSizeLe function", () => {
    it("should return a valid value for initial buffer", async () => {
      const buf = Buffer.from(
        "0000000000000000000000000000000000000000000000000000000000000000",
        "hex"
      );
      const result = await hashToBn254FieldSizeLe(buf);
      expect(result).not.toBeNull();
      if (result) {
        expect(result[0]).toBeInstanceOf(Buffer);
        expect(result[1]).toBe(255);
      }
    });

    it("should return a valid value for a buffer that can be hashed to a smaller value", async () => {
      const buf = Buffer.from(
        "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
        "hex"
      );
      const result = await hashToBn254FieldSizeLe(buf);
      expect(result).not.toBeNull();
      if (result) {
        expect(result[1]).toBeLessThanOrEqual(255);
        expect(result[0]).toBeInstanceOf(Buffer);
        // Check if the hashed value is indeed smaller than the bn254 field size
        expect(isSmallerThanBn254FieldSizeLe(result[0])).toBe(true);
      }
    });

    it("should correctly hash the input buffer", async () => {
      const buf = Buffer.from("deadbeef", "hex");
      const result = await hashToBn254FieldSizeLe(buf);
      expect(result).not.toBeNull();
      if (result) {
        // Since the actual hash value depends on the crypto implementation and input,
        // we cannot predict the exact output. However, we can check if the output is valid.
        expect(result[0].length).toBe(32); // SHA-256 hash length
        expect(result[1]).toBeLessThanOrEqual(255);
        expect(isSmallerThanBn254FieldSizeLe(result[0])).toBe(true);
      }
    });
  });

  describe("pushUniqueItems function", () => {
    it("should add unique items", () => {
      const map = [1, 2, 3];
      const itemsToAdd = [3, 4, 5];
      pushUniqueItems(itemsToAdd, map);
      expect(map).toEqual([1, 2, 3, 4, 5]);
    });

    it("should ignore duplicates", () => {
      const map = [1, 2, 3];
      const itemsToAdd = [1, 2, 3];
      pushUniqueItems(itemsToAdd, map);
      expect(map).toEqual([1, 2, 3]);
    });

    it("should handle empty arrays", () => {
      const map: number[] = [];
      const itemsToAdd: number[] = [];
      pushUniqueItems(itemsToAdd, map);
      expect(map).toEqual([]);
    });
  });
  describe("bigintToArray", () => {
    it("should convert 0 to [0]", () => {
      expect(bigintToArray(BigInt(0))).toEqual([0]);
    });

    it("should convert 1 to [1]", () => {
      expect(bigintToArray(BigInt(1))).toEqual([1]);
    });

    it("should convert 256 to [1, 0]", () => {
      expect(bigintToArray(BigInt(256))).toEqual([1, 0]);
    });

    it("should convert 257 to [1, 1]", () => {
      expect(bigintToArray(BigInt(257))).toEqual([1, 1]);
    });

    it("should convert 123456789 to [7, 91, 205, 21]", () => {
      expect(bigintToArray(BigInt(123456789))).toEqual([7, 91, 205, 21]);
    });
  });

  describe("arrayToBigint", () => {
    it("should convert [0] to BigInt(0)", () => {
      expect(arrayToBigint([0])).toEqual(BigInt(0));
    });

    it("should convert [1] to BigInt(1)", () => {
      expect(arrayToBigint([1])).toEqual(BigInt(1));
    });

    it("should convert [1, 0] to BigInt(256)", () => {
      expect(arrayToBigint([1, 0])).toEqual(BigInt(256));
    });

    it("should convert [1, 1] to BigInt(257)", () => {
      expect(arrayToBigint([1, 1])).toEqual(BigInt(257));
    });

    it("should convert [7, 91, 205, 21] to BigInt(123456789)", () => {
      expect(arrayToBigint([7, 91, 205, 21])).toEqual(BigInt(123456789));
    });
  });

  describe("bufToDecStr", () => {
    it("should convert buffer [0] to '0'", () => {
      expect(bufToDecStr(Buffer.from([0]))).toEqual("0");
    });

    it("should convert buffer [1] to '1'", () => {
      expect(bufToDecStr(Buffer.from([1]))).toEqual("1");
    });

    it("should convert buffer [1, 0] to '256'", () => {
      expect(bufToDecStr(Buffer.from([1, 0]))).toEqual("256");
    });

    it("should convert buffer [1, 1] to '257'", () => {
      expect(bufToDecStr(Buffer.from([1, 1]))).toEqual("257");
    });

    it("should convert buffer [7, 91, 205, 21] to '123456789'", () => {
      expect(bufToDecStr(Buffer.from([7, 91, 205, 21]))).toEqual("123456789");
    });
  });
}
