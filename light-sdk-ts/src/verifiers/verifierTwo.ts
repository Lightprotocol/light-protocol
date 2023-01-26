// @ts-nocheck
import {
  VerifierProgramTwo,
  VerifierProgramTwoIdl,
} from "../idls/verifier_program_two";
import { Program } from "@coral-xyz/anchor";
import {
  hashAndTruncateToCircuit,
  verifierProgramTwoProgramId,
} from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
export class VerifierTwo implements Verifier {
  verifierProgram: Program<VerifierProgramTwoIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  nrPublicInputs: number;
  config: { in: number; out: number };
  pubkey: BN;
  constructor() {
    this.verifierProgram = new Program(
      VerifierProgramTwo,
      verifierProgramTwoProgramId,
    );

    this.wtnsGenPath = "transactionApp4_js/transactionApp4.wasm";
    this.zkeyPath = "transactionApp4.zkey";
    this.calculateWtns = require("../../build-circuits/transactionApp4_js/witness_calculator.js");
    this.nrPublicInputs = 15;
    this.config = { in: 4, out: 4 };
    this.pubkey = hashAndTruncateToCircuit(
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").toBytes(),
    );
  }

  parsePublicInputsFromArray(publicInputsBytes: any): PublicInputs {
    if (publicInputsBytes.length == this.nrPublicInputs) {
      return {
        root: publicInputsBytes[0],
        publicAmount: publicInputsBytes[1],
        extDataHash: publicInputsBytes[2],
        feeAmount: publicInputsBytes[3],
        mintPubkey: publicInputsBytes[4],
        nullifiers: Array.from(publicInputsBytes.slice(5, 9)),
        leaves: Array.from([
          publicInputsBytes.slice(9, 11),
          publicInputsBytes.slice(11, 13),
        ]),
        checkedParams: Array.from(publicInputsBytes.slice(13, 15)),
        connectingHash: Array.from(publicInputsBytes.slice(13, 14)),
        verifier: Array.from(publicInputsBytes.slice(14, 15)),
      };
    } else {
      throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.nrPublicInputs}`;
    }
  }

  initVerifierProgram(): void {
    this.verifierProgram = new Program(
      VerifierProgramTwo,
      verifierProgramTwoProgramId,
    );
  }

  // Do I need a getData fn?
  // I should be able to fetch everything from the object
  async getInstructions(transaction: Transaction): Promise<any> {
    console.log("empty is cpi");
  }
}
