import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { verifierProgramZeroProgramId } from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import {
  VerifierProgramZero,
  VerifierProgramZeroIdl,
} from "../idls/verifier_program_zero";
// Proofgen does not work within sdk needs circuit-build
// TODO: bundle files in npm package
// TODO: define verifier with an Idl thus absorb this functionality into the Transaction class
export class VerifierZero implements Verifier {
  verifierProgram: Program<VerifierProgramZeroIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number };
  instructions?: anchor.web3.TransactionInstruction[];
  constructor() {
    try {
      this.verifierProgram = new Program(
        VerifierProgramZero,
        verifierProgramZeroProgramId
      );
    } catch (error) {
      
    }

    this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
    this.zkeyPath = `./build-circuits/transactionMasp2`;
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.config = { in: 2, out: 2 };
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {
    if (transaction.publicInputsBytes) {
      if (transaction.publicInputsBytes.length == 9) {
        return {
          root: transaction.publicInputsBytes[0],
          publicAmount: transaction.publicInputsBytes[1],
          extDataHash: transaction.publicInputsBytes[2],
          feeAmount: transaction.publicInputsBytes[3],
          mintPubkey: transaction.publicInputsBytes[4],
          nullifiers: [
            transaction.publicInputsBytes[5],
            transaction.publicInputsBytes[6],
          ],
          leaves: [
            [
              transaction.publicInputsBytes[7],
              transaction.publicInputsBytes[8],
            ],
          ],
        };
      } else {
        throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 9`;
      }
    } else {
      throw new Error("public input bytes undefined");
    }
  }

  async getInstructions(
    transaction: Transaction
  ): Promise<anchor.web3.TransactionInstruction[]> {
    if (
      transaction.params &&
      transaction.params.nullifierPdaPubkeys &&
      transaction.params.leavesPdaPubkeys
    ) {
      if (!transaction.payer) {
        throw new Error("Payer not defined");
      }

      const ix = await this.verifierProgram.methods
        .shieldedTransferInputs(
          Buffer.from(transaction.proofBytes),
          Buffer.from(transaction.publicInputs.publicAmount),
          transaction.publicInputs.nullifiers,
          transaction.publicInputs.leaves[0],
          Buffer.from(transaction.publicInputs.feeAmount),
          new anchor.BN(transaction.rootIndex.toString()),
          new anchor.BN(transaction.relayer.relayerFee.toString()),
          Buffer.from(transaction.encryptedUtxos.slice(0, 190)) // remaining bytes can be used once tx sizes increase
        )
        .accounts({
          ...transaction.params.accounts,
          ...transaction.relayer.accounts,
        })
        .remainingAccounts([
          ...transaction.params.nullifierPdaPubkeys,
          ...transaction.params.leavesPdaPubkeys,
        ])
        .instruction();
      this.instructions = [ix];
      return [ix];
    } else {
      throw new Error(
        "transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined"
      );
    }
  }
}
