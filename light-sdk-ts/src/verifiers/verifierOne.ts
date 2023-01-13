import {
  VerifierProgramOne,
  VerifierProgramOneIdl,
} from "../idls/verifier_program_one";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { verifierProgramOneProgramId } from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";

export class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOneIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number };
  instructions?: anchor.web3.TransactionInstruction[];

  constructor() {
    this.verifierProgram = new Program(
      VerifierProgramOne,
      verifierProgramOneProgramId
    );
    this.wtnsGenPath =
      "./build-circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = "./build-circuits/transactionMasp10";
    this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
    this.config = { in: 10, out: 2 };
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {
    if (transaction.publicInputsBytes.length == 17) {
      return {
        root: transaction.publicInputsBytes[0],
        publicAmount: transaction.publicInputsBytes[1],
        extDataHash: transaction.publicInputsBytes[2],
        feeAmount: transaction.publicInputsBytes[3],
        mintPubkey: transaction.publicInputsBytes[4],
        nullifiers: Array.from(transaction.publicInputsBytes.slice(5, 15)),
        leaves: [
          [
            transaction.publicInputsBytes[15],
            transaction.publicInputsBytes[16],
          ],
        ],
      };
    } else {
      throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 17`;
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
      const ix1 = await this.verifierProgram.methods
        .shieldedTransferFirst(
          Buffer.from(transaction.publicInputs.publicAmount),
          transaction.publicInputs.nullifiers,
          transaction.publicInputs.leaves[0],
          Buffer.from(transaction.publicInputs.feeAmount),
          new anchor.BN(transaction.rootIndex.toString()),
          new anchor.BN(transaction.relayer.relayerFee.toString()),
          Buffer.from(transaction.encryptedUtxos)
        )
        .accounts({
          ...transaction.params.accounts,
          ...transaction.relayer.accounts,
        })
        .instruction();

      const ix2 = await this.verifierProgram.methods
        .shieldedTransferSecond(Buffer.from(transaction.proofBytes))
        .accounts({
          ...transaction.params.accounts,
          ...transaction.relayer.accounts,
        })
        .remainingAccounts([
          ...transaction.params.nullifierPdaPubkeys,
          ...transaction.params.leavesPdaPubkeys,
        ])
        .signers([transaction.payer])
        .instruction();
      this.instructions = [ix1, ix2];
      return this.instructions;
    } else {
      throw new Error(
        "transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined"
      );
    }
  }
}
