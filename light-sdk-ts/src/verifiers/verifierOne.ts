import { VerifierProgramOne, IDL_VERIFIER_PROGRAM_ONE } from "../idls/index";
import * as anchor from "@coral-xyz/anchor";
import { BorshAccountsCoder, Program } from "@coral-xyz/anchor";

import {
  createAccountObject,
  firstLetterToLower,
  firstLetterToUpper,
  hashAndTruncateToCircuit,
  TransactionErrorCode,
  VerifierError,
  VerifierErrorCode,
  verifierProgramOneProgramId,
} from "../index";
import { Transaction } from "transaction";
import { Verifier, PublicInputs, VerifierConfig } from ".";

export class VerifierOne implements Verifier {
  verifierProgram?: Program<VerifierProgramOne>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: VerifierConfig;
  instructions?: anchor.web3.TransactionInstruction[];
  pubkey: anchor.BN;
  idl: anchor.Idl;
  constructor() {
    try {
      this.verifierProgram = new Program(
        IDL_VERIFIER_PROGRAM_ONE,
        verifierProgramOneProgramId,
      );
    } catch (error) {
      console.log(error);
    }
    this.wtnsGenPath = "transactionMasp10_js/transactionMasp10.wasm";
    this.zkeyPath = "transactionMasp10.zkey";
    this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
    this.config = { in: 10, out: 2, nrPublicInputs: 17, isAppVerifier: false };
    this.pubkey = hashAndTruncateToCircuit(
      verifierProgramOneProgramId.toBytes(),
    );
    this.idl = IDL_VERIFIER_PROGRAM_ONE;
  }

  parsePublicInputsFromArray(
    publicInputsBytes: Array<Array<number>>,
  ): PublicInputs {
    if (!publicInputsBytes) {
      throw new VerifierError(
        VerifierErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "parsePublicInputsFromArray",
        "verifier one:",
      );
    }
    if (publicInputsBytes.length != this.config.nrPublicInputs) {
      throw new VerifierError(
        VerifierErrorCode.INVALID_INPUTS_NUMBER,
        "parsePublicInputsFromArray",
        `verifier one: publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.config.nrPublicInputs}`,
      );
    }
    return {
      root: publicInputsBytes[0],
      publicAmountSpl: publicInputsBytes[1],
      txIntegrityHash: publicInputsBytes[2],
      publicAmountSol: publicInputsBytes[3],
      publicMintPubkey: publicInputsBytes[4],
      nullifiers: Array.from(publicInputsBytes.slice(5, 15)),
      leaves: [publicInputsBytes[15], publicInputsBytes[16]],
    };
  }
}
