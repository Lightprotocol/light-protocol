/// <reference types="node" />
import { VerifierProgramOneIdl } from "../idls/verifier_program_one";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOneIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor();
  parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
  initVerifierProgram(): void;
  transferFirst(transfer: Transaction): Promise<void>;
  transferSecond(transfer: Transaction): Promise<unknown>;
  sendTransaction(transfer: Transaction): Promise<any>;
}
