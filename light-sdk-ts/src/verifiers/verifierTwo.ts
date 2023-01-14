import {
  VerifierProgramTwo,
  VerifierProgramTwoIdl,
} from "../idls/verifier_program_two";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
} from "@solana/web3.js";
import {
  DEFAULT_PROGRAMS,
  REGISTERED_VERIFIER_TWO_PDA,
  verifierProgramTwoProgramId,
} from "../index";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { assert } from "chai";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { verifierProgramTwo } from "../index";

export class VerifierTwo implements Verifier {
  verifierProgram: Program<VerifierProgramTwoIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  nrPublicInputs: number;
  config: {in: number, out: number}
  constructor() {
    this.verifierProgram = new Program(
      VerifierProgramTwo,
      verifierProgramTwoProgramId,
    );

    this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
    this.zkeyPath = "./build-circuits/transactionMasp2";
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.registeredVerifierPda = REGISTERED_VERIFIER_TWO_PDA;
    this.nrPublicInputs = 17;
    this.config = {in: 4, out: 4};
    console.log("TODO Change paths to 4 ins 4 outs circuit");
    console.log("REGISTERED_VERIFIER_TWO_PDA: is ONE");
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {
    if (transaction.publicInputsBytes.length == this.nrPublicInputs) {
      return {
        root: transaction.publicInputsBytes[0],
        publicAmount: transaction.publicInputsBytes[1],
        extDataHash: transaction.publicInputsBytes[2],
        feeAmount: transaction.publicInputsBytes[3],
        mintPubkey: transaction.publicInputsBytes[4],
        checkedParams: Array.from(transaction.publicInputsBytes.slice(5, 9)),
        nullifiers: Array.from(transaction.publicInputsBytes.slice(9, 13)),
        leaves: Array.from(
          transaction.publicInputsBytes.slice(13, this.nrPublicInputs),
        ),
      };
    } else {
      throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != ${this.nrPublicInputs}`;
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
