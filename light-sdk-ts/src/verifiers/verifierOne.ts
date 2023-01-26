// @ts-nocheck
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
    try {
      this.verifierProgram = new Program(
        VerifierProgramOne,
        verifierProgramOneProgramId,
      );
    } catch (e) {}
    this.wtnsGenPath =
      "./build-circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = "./build-circuits/transactionMasp10";
    this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
    this.config = { in: 10, out: 2 };
  }

  parsePublicInputsFromArray(publicInputsBytes: any): PublicInputs {
    if (publicInputsBytes.length == 17) {
      return {
        root: publicInputsBytes[0],
        publicAmount: publicInputsBytes[1],
        extDataHash: publicInputsBytes[2],
        feeAmount: publicInputsBytes[3],
        mintPubkey: publicInputsBytes[4],
        nullifiers: Array.from(publicInputsBytes.slice(5, 15)),
        leaves: [[publicInputsBytes[15], publicInputsBytes[16]]],
      };
    } else {
      throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != 17`;
    }
  }

  async getInstructions(
    transaction: Transaction,
  ): Promise<anchor.web3.TransactionInstruction[]> {
    if (
      transaction.params &&
      transaction.params.nullifierPdaPubkeys &&
      transaction.params.leavesPdaPubkeys &&
      transaction.publicInputs
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
          Buffer.from(transaction.encryptedUtxos),
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
        "transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined",
      );
    }
  }
}
