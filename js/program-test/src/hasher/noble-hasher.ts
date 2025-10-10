import type BN from "bn.js";
import * as mod from "@noble/curves/abstract/modular.js";
import * as poseidon from "@noble/curves/abstract/poseidon.js";
import { LightWasm } from "../test-rpc/test-rpc";
import { bn } from "@lightprotocol/stateless.js";
import { CONSTANTS_3_FLAT, CONSTANTS_4_FLAT, MDS_3, MDS_4 } from "./constants";

/**
 * Noble Poseidon hasher implementation that replaces the WASM-based hasher.
 *
 * This implementation uses @noble/curves Poseidon with Circom-compatible parameters:
 * - Field: BN254 (alt_bn128)
 * - State size: t=3 (for 2 inputs), t=4 (for 3 inputs)
 * - Rounds: 8 full + 57 partial (t=3), 8 full + 56 partial (t=4)
 * - S-box: x^5
 * - Constants from Light Protocol's constants.go
 */

// BN254 field modulus (alt_bn128)
const BN254_MODULUS = BigInt(
  "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001",
);
const Fp = mod.Field(BN254_MODULUS);

// Capacity element for Poseidon (first element in state array)
const POSEIDON_CAPACITY = BigInt(0);

// Initialize Poseidon hash function for t=3 (2 inputs)
const roundConstants3 = poseidon.splitConstants(CONSTANTS_3_FLAT, 3);
const poseidonNoble3 = poseidon.poseidon({
  Fp,
  t: 3,
  roundsFull: 8,
  roundsPartial: 57,
  sboxPower: 5,
  mds: MDS_3,
  roundConstants: roundConstants3,
});

// Initialize Poseidon hash function for t=4 (3 inputs)
const roundConstants4 = poseidon.splitConstants(CONSTANTS_4_FLAT, 4);
const poseidonNoble4 = poseidon.poseidon({
  Fp,
  t: 4,
  roundsFull: 8,
  roundsPartial: 56,
  sboxPower: 5,
  mds: MDS_4,
  roundConstants: roundConstants4,
});

/**
 * Convert input (string[] | BN[]) to bigint array
 */
function toBigIntArray(input: string[] | BN[]): bigint[] {
  return input.map((val) => {
    if (typeof val === "string") {
      return BigInt(val);
    } else {
      // BN type - use toString(10) to ensure decimal representation
      const str = val.toString(10);
      if (!str || str === "NaN" || str.includes("NaN")) {
        throw new Error(`Invalid BN value: ${str}`);
      }
      return BigInt(str);
    }
  });
}

/**
 * Convert bigint to Uint8Array (32 bytes, big-endian)
 */
function bigintToUint8Array(value: bigint): Uint8Array {
  const hex = value.toString(16).padStart(64, "0");
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

/**
 * Noble Poseidon hasher that implements the LightWasm interface
 */
export class NobleHasher implements LightWasm {
  /**
   * Poseidon hash returning Uint8Array
   * @param input Array of 2 or 3 inputs (as strings or BN)
   * @returns 32-byte hash as Uint8Array
   */
  poseidonHash(input: string[] | BN[]): Uint8Array {
    const inputs = toBigIntArray(input);
    let hash: bigint;

    if (inputs.length === 2) {
      // Use t=3 Poseidon: [CAPACITY, input1, input2]
      const state = poseidonNoble3([POSEIDON_CAPACITY, inputs[0], inputs[1]]);
      hash = state[0];
    } else if (inputs.length === 3) {
      // Use t=4 Poseidon: [CAPACITY, input1, input2, input3]
      const state = poseidonNoble4([
        POSEIDON_CAPACITY,
        inputs[0],
        inputs[1],
        inputs[2],
      ]);
      hash = state[0];
    } else {
      throw new Error(`Expected 2 or 3 inputs, got ${inputs.length}`);
    }

    return bigintToUint8Array(hash);
  }

  /**
   * Poseidon hash returning string (decimal representation)
   * @param input Array of 2 or 3 inputs (as strings or BN)
   * @returns Hash as decimal string
   */
  poseidonHashString(input: string[] | BN[]): string {
    const inputs = toBigIntArray(input);
    let hash: bigint;

    if (inputs.length === 2) {
      // Use t=3 Poseidon: [CAPACITY, input1, input2]
      const state = poseidonNoble3([POSEIDON_CAPACITY, inputs[0], inputs[1]]);
      hash = state[0];
    } else if (inputs.length === 3) {
      // Use t=4 Poseidon: [CAPACITY, input1, input2, input3]
      const state = poseidonNoble4([
        POSEIDON_CAPACITY,
        inputs[0],
        inputs[1],
        inputs[2],
      ]);
      hash = state[0];
    } else {
      throw new Error(`Expected 2 or 3 inputs, got ${inputs.length}`);
    }

    return hash.toString();
  }

  /**
   * Poseidon hash returning BN (bn.js instance)
   * @param input Array of 2 or 3 inputs (as strings or BN)
   * @returns Hash as BN
   */
  poseidonHashBN(input: string[] | BN[]): BN {
    const inputs = toBigIntArray(input);
    let hash: bigint;

    if (inputs.length === 2) {
      // Use t=3 Poseidon: [CAPACITY, input1, input2]
      const state = poseidonNoble3([POSEIDON_CAPACITY, inputs[0], inputs[1]]);
      hash = state[0];
    } else if (inputs.length === 3) {
      // Use t=4 Poseidon: [CAPACITY, input1, input2, input3]
      const state = poseidonNoble4([
        POSEIDON_CAPACITY,
        inputs[0],
        inputs[1],
        inputs[2],
      ]);
      hash = state[0];
    } else {
      throw new Error(`Expected 2 or 3 inputs, got ${inputs.length}`);
    }

    return bn(hash.toString());
  }
}

/**
 * Factory for creating Noble hasher instances
 * Mirrors the WasmFactory.getInstance() API for drop-in replacement
 */
export class NobleHasherFactory {
  private static instance: NobleHasher | null = null;

  /**
   * Get singleton instance of Noble hasher
   * @returns NobleHasher instance (implements LightWasm interface)
   */
  static async getInstance(): Promise<NobleHasher> {
    if (!this.instance) {
      this.instance = new NobleHasher();
    }
    return this.instance;
  }

  /**
   * Synchronous version for contexts where async is not needed
   * @returns NobleHasher instance
   */
  static getInstanceSync(): NobleHasher {
    if (!this.instance) {
      this.instance = new NobleHasher();
    }
    return this.instance;
  }
}
