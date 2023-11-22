import { poseidon as wasmPoseidon } from "./wasm/light_wasm";
import { featureFlags } from "./config";
import { BN } from "@coral-xyz/anchor";
const circomlibjs = require("circomlibjs");

export interface IHasher {
  hash(input: string[] | BN[]): Uint8Array;
  hashString(input: string[] | BN[]): string;
  hashBN(input: string[] | BN[]): BN;
}

export class Poseidon implements IHasher {
  private static instance: Poseidon;

  private circomPoseidon: any | undefined;
  private constructor() {}

  public static async getInstance(): Promise<Poseidon> {
    if (!Poseidon.instance) {
      Poseidon.instance = new Poseidon();
    }
    if (!featureFlags.wasmPoseidon) {
      Poseidon.instance.circomPoseidon = await circomlibjs.buildPoseidonOpt();
    }
    return Poseidon.instance;
  }
  public hash(input: string[] | BN[]): Uint8Array {
    if (featureFlags.wasmPoseidon) {
      return wasmPoseidon(this.stringify(input));
    } else {
      return this.circomPoseidon.poseidon(input);
    }
  }

  public hashBN(input: string[] | BN[]): BN {
    return new BN(this.hash(input));
  }

  public hashString(input: string[] | BN[]): string {
    return this.string(this.hash(input));
  }

  private stringify(input: string[] | BN[]): string[] {
    if (input.length > 0 && input[0] instanceof BN) {
      return (input as BN[]).map((item) => item.toString(10));
    } else {
      return input as string[];
    }
  }
  private string(hash: Uint8Array): string {
    if (featureFlags.wasmPoseidon) {
      const bn = new BN(hash);
      return bn.toString();
    } else {
      return this.circomPoseidon.F.toString(hash);
    }
  }
}
