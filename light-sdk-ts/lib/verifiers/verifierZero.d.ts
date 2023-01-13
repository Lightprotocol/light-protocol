/// <reference types="node" />
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { VerifierProgramZeroIdl } from "../idls/verifier_program_zero";
export declare class VerifierZero implements Verifier {
  verifierProgram: Program<VerifierProgramZeroIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor();
  parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
  initVerifierProgram(): void;
  sendTransaction(transaction: Transaction): Promise<any>;
}
