export * from "./verifierOne";
export * from "./verifierZero";
export * from "./verifierTwo";

import { Program, web3, BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";

export type PublicInputs = {
  root: Array<Number>;
  publicAmount: Buffer;
  extDataHash: Array<Number>;
  feeAmount: Buffer;
  mintPubkey: Array<Number>;
  nullifiers: Array<Array<Number>>;
  leaves: Array<Array<Array<Number>>>;
  // only for app verifiers
  connectingHash?: Array<Number>;
  checkedParams?: Array<Array<Number>>;
  verifier?: Array<number>;
};

export type VerifierConfig = {
  in: number;
  out: number;
  nrPublicInputs: number;
};

export interface Verifier {
  verifierProgram?: Program<any>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number };
  instructions?: web3.TransactionInstruction[];
  parsePublicInputsFromArray(
    publicInputsBytes: Array<Array<number>>,
  ): PublicInputs;
  getInstructions(
    transaction: Transaction,
  ): Promise<web3.TransactionInstruction[]>;
  pubkey?: BN;
}
