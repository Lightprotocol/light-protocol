/// <reference types="node" />
import { VerifierProgramOne } from "../../idls/verifier_program_one";
import { Program } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOne>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor();
  parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
  transferFirst(transfer: Transaction): Promise<void>;
  transferSecond(transfer: Transaction): Promise<unknown>;
  sendTransaction(insert?: Boolean): Promise<any>;
}
