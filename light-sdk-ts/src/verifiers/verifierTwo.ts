import { VerifierProgramTwo } from "../../idls/verifier_program_one";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {Connection, PublicKey, Keypair, SystemProgram, TransactionMessage, ComputeBudgetProgram,  AddressLookupTableAccount, VersionedTransaction, sendAndConfirmRawTransaction } from "@solana/web3.js";
import {
  DEFAULT_PROGRAMS, REGISTERED_VERIFIER_TWO_PDA,
} from "../constants";
import { TOKEN_PROGRAM_ID, getAccount  } from '@solana/spl-token';
import { assert } from "chai";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import {verifierProgramTwo } from "../constants"

export class VerifierTwo implements Verifier {
  verifierProgram: Program<VerifierProgramTwo>
  wtnsGenPath: String
  zkeyPath: String
  calculateWtns: NodeRequire
  registeredVerifierPda: PublicKey
  nrPublicInputs: number
  constructor() {
    this.verifierProgram = verifierProgramTwo;
    this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
    this.zkeyPath = './build-circuits/transactionMasp2'
    this.calculateWtns = require('../../build-circuits/transactionMasp2_js/witness_calculator.js')
    this.registeredVerifierPda =  REGISTERED_VERIFIER_TWO_PDA
    this.nrPublicInputs = 17;
    console.log("TODO Change paths to 4 ins 4 outs circuit");
    console.log("REGISTERED_VERIFIER_TWO_PDA: is ONE");
    
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {

    if (transaction.publicInputsBytes.length == this.nrPublicInputs) {
        return {
         root:         transaction.publicInputsBytes[0],
         publicAmount: transaction.publicInputsBytes[1],
         extDataHash:  transaction.publicInputsBytes[2],
         feeAmount:    transaction.publicInputsBytes[3],
         mintPubkey:   transaction.publicInputsBytes[4],
         checkedParams:   Array.from(transaction.publicInputsBytes.slice(5,9)),
         nullifiers:   Array.from(transaction.publicInputsBytes.slice(9,13)),
         leaves:     Array.from(transaction.publicInputsBytes.slice(13,this.nrPublicInputs))
       };
    } else {
      throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != ${this.nrPublicInputs}`;
    }

  }

  // Do I need a getData fn?
  // I should be able to fetch everything from the object
  async sendTransaction(insert: Boolean): Promise<any> {
    console.log("empty is cpi");
    
  }
}
