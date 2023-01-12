/// <reference types="node" />
import { VerifierProgramZero } from "../../idls/verifier_program_zero";
import { Program } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierZero implements Verifier {
  verifierProgram: Program<VerifierProgramZero>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor();
  parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
  sendTransaction(insert?: Boolean): Promise<any>;
}
