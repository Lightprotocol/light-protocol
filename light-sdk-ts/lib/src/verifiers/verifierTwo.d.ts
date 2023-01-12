/// <reference types="node" />
import { VerifierProgramTwo } from "../../idls/verifier_program_one";
import { Program } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierTwo implements Verifier {
  verifierProgram: Program<VerifierProgramTwo>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  nrPublicInputs: number;
  constructor();
  parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
  sendTransaction(insert: Boolean): Promise<any>;
}
