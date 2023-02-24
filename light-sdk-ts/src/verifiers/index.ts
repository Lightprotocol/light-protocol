export * from "./verifierOne";
export * from "./verifierZero";
export * from "./verifierTwo";

import { Program, web3, BN } from "@coral-xyz/anchor";
import { Transaction } from "../transaction";

export type PublicInputs = {
  root: Array<number>;
  publicAmount: Array<number>;
  extDataHash: Array<number>;
  feeAmount: Array<number>;
  mintPubkey: Array<number>;
  nullifiers: Array<Array<number>>;
  leaves: Array<Array<Array<number>>>;
  // only for app verifiers
  connectingHash?: Array<number>;
  checkedParams?: Array<Array<number>>;
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
