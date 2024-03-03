import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { FIELD_SIZE } from "../constants";
import { PublicKey } from "@solana/web3.js";

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
export function bigint254toBase58(bigintNumber: bigint254, pad = 32): string {
  let buffer = Buffer.from(bigintNumber.toString(16), "hex");
  // Ensure the buffer is 32 bytes. If not, pad it with leading zeros.
  if (buffer.length < pad) {
    const padding = Buffer.alloc(pad - buffer.length);
    buffer = Buffer.concat([padding, buffer], pad);
  }
  return bs58.encode(buffer);
}
/** Convert Base58 string to <254-bit Solana Public key*/
export function bigint254ToPublicKey(bigintNumber: bigint254): PublicKey {
  const paddedBase58 = bigint254toBase58(bigintNumber);
  return new PublicKey(paddedBase58);
}

// FIXME: assumes <254 bit pubkey.
// just use consistent type (pubkey254)
/** Convert Solana Public key to <254-bit bigint */
export function PublicKeyToBigint254(publicKey: PublicKey): bigint254 {
  const buffer = publicKey.toBuffer();
  // Remove leading zeros from the buffer
  const trimmedBuffer = buffer.subarray(buffer.findIndex((byte) => byte !== 0));
  return createBigint254(trimmedBuffer);
}

//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { it, expect, describe } = import.meta.vitest;

  describe("createBigint254 function", () => {
    it("should create a bigint254 from a string", () => {
      const bigint = createBigint254("100");
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a number", () => {
      const bigint = createBigint254(100);
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a bigint", () => {
      const bigint = createBigint254(BigInt(100));
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a Buffer", () => {
      const bigint = createBigint254(Buffer.from([100]));
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a Uint8Array", () => {
      const bigint = createBigint254(new Uint8Array([100]));
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a number[]", () => {
      const bigint = createBigint254([100]);
      expect(bigint).toBe(BigInt(100));
    });

    it("should create a bigint254 from a base58 string", () => {
      const bigint = createBigint254("2j", "base58");
      expect(bigint).toBe(BigInt(100));
    });
  });

  describe("bigint254toBase58 function", () => {
    it("should convert a bigint254 to a base58 string, no pad", () => {
      const bigint = createBigint254("100");
      const base58 = bigint254toBase58(bigint, 0);
      expect(base58).toBe("2j");
    });

    it("should convert a bigint254 to a base58 string, with pad", () => {
      const bigint = createBigint254("100");
      const base58 = bigint254toBase58(bigint);
      expect(base58).toBe("11111111111111111111111111111112j");
    });
    it("should throw an error for a value that is too large", () => {
      expect(() => createBigint254(FIELD_SIZE)).toThrow(
        "Value is too large. Max <254 bits"
      );
    });
  });

  describe("bigint254ToPublicKey function", () => {
    it("should convert a bigint254 to a PublicKey", () => {
      const bigint = createBigint254("100");
      const publicKey = bigint254ToPublicKey(bigint);
      expect(publicKey).toBeInstanceOf(PublicKey);
    });
  });

  describe("PublicKeyToBigint254 function", () => {
    it("should convert a PublicKey to a bigint254", () => {
      const publicKey = PublicKey.unique();
      const bigint = PublicKeyToBigint254(publicKey);
      expect(typeof bigint).toBe("bigint");
    });
  });
}
