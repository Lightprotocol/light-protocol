import * as anchor from "@coral-xyz/anchor";
import { BorshAccountsCoder, Program } from "@coral-xyz/anchor";
import {
  hashAndTruncateToCircuit,
  Provider,
  TransactionErrorCode,
  VerifierError,
  VerifierErrorCode,
  verifierProgramZeroProgramId,
  createAccountObject,
} from "../index";
import { Transaction } from "transaction";
import { Verifier, PublicInputs, VerifierConfig } from ".";
import { VerifierProgramZero, IDL_VERIFIER_PROGRAM_ZERO } from "../idls/index";
import { IDL } from "@coral-xyz/anchor/dist/cjs/native/system";

// TODO: define verifier with an Idl thus absorb this functionality into the Transaction class
export class VerifierZero implements Verifier {
  verifierProgram?: Program<VerifierProgramZero>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: VerifierConfig;
  instructions?: anchor.web3.TransactionInstruction[];
  pubkey: anchor.BN;
  idl: anchor.Idl;
  constructor(provider?: Provider) {
    try {
      this.verifierProgram = new Program(
        IDL_VERIFIER_PROGRAM_ZERO,
        verifierProgramZeroProgramId,
        // @ts-ignore
        provider,
      );
    } catch (error) {
      console.log(error);
    }
    // ./build-circuits/transactionMasp2_js/
    this.wtnsGenPath = "transactionMasp2_js/transactionMasp2.wasm";
    this.zkeyPath = `transactionMasp2.zkey`;
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.config = { in: 2, out: 2, nrPublicInputs: 9, isAppVerifier: false };
    this.pubkey = hashAndTruncateToCircuit(
      verifierProgramZeroProgramId.toBytes(),
    );
    this.idl = IDL_VERIFIER_PROGRAM_ZERO;
  }

  parsePublicInputsFromArray(
    publicInputsBytes: Array<Array<number>>,
  ): PublicInputs {
    if (!publicInputsBytes) {
      throw new VerifierError(
        VerifierErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "parsePublicInputsFromArray",
        "verifier zero",
      );
    }
    if (publicInputsBytes.length != this.config.nrPublicInputs) {
      throw new VerifierError(
        VerifierErrorCode.INVALID_INPUTS_NUMBER,
        "parsePublicInputsFromArray",
        `verifier zero: publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.config.nrPublicInputs}`,
      );
    }
    return {
      root: publicInputsBytes[0],
      publicAmountSpl: publicInputsBytes[1],
      txIntegrityHash: publicInputsBytes[2],
      publicAmountSol: publicInputsBytes[3],
      publicMintPubkey: publicInputsBytes[4],
      nullifiers: [publicInputsBytes[5], publicInputsBytes[6]],
      leaves: [publicInputsBytes[7], publicInputsBytes[8]],
    };
  }
}
