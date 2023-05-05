export * from "./verifierOne";
export * from "./verifierZero";
export * from "./verifierTwo";

import { Program, web3, BN, Idl } from "@coral-xyz/anchor";
import { Transaction } from "transaction";

export type PublicInputs = {
  root: Array<number>;
  publicAmountSpl: Array<number>;
  txIntegrityHash: Array<number>;
  publicAmountSol: Array<number>;
  publicMintPubkey: Array<number>;
  nullifiers: Array<Array<number>>;
  leaves: Array<Array<number>>;
  // only for app verifiers
  transactionHash?: Array<number>;
  checkedParams?: Array<Array<number>>;
  publicAppVerifier?: Array<number>;
};

export type VerifierConfig = {
  in: number;
  out: number;
  nrPublicInputs: number;
  isAppVerifier: boolean;
};

export interface Verifier {
  verifierProgram?: Program<any>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number; isAppVerifier: boolean };
  instructions?: web3.TransactionInstruction[];
  parsePublicInputsFromArray(
    publicInputsBytes: Array<Array<number>>,
  ): PublicInputs;
  pubkey?: BN;
  idl: Idl;
}
