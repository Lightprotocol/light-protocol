import { VerifierProgramOne, IDL_VERIFIER_PROGRAM_ONE } from "../idls/index";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import {
  hashAndTruncateToCircuit,
  verifierProgramOneProgramId,
} from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs, VerifierConfig } from ".";

export class VerifierOne implements Verifier {
  verifierProgram?: Program<VerifierProgramOne>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: VerifierConfig;
  instructions?: anchor.web3.TransactionInstruction[];
  pubkey: anchor.BN;

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
    this.config = { in: 10, out: 2, nrPublicInputs: 17 };
    this.pubkey = hashAndTruncateToCircuit(
      verifierProgramOneProgramId.toBytes(),
    );
  }

  parsePublicInputsFromArray(publicInputsBytes: any): PublicInputs {
    if (publicInputsBytes.length != 17) {
      throw new Error(
        `publicInputsBytes.length invalid ${publicInputsBytes.length} != 17`,
      );
    }
    return {
      root: publicInputsBytes[0],
      publicAmount: publicInputsBytes[1],
      extDataHash: publicInputsBytes[2],
      feeAmount: publicInputsBytes[3],
      mintPubkey: publicInputsBytes[4],
      nullifiers: Array.from(publicInputsBytes.slice(5, 15)),
      leaves: [[publicInputsBytes[15], publicInputsBytes[16]]],
    };
  }

  async getInstructions(
    transaction: Transaction,
  ): Promise<anchor.web3.TransactionInstruction[]> {
    if (!transaction.params) throw new Error("params undefined");
    if (!transaction.remainingAccounts)
      throw new Error("remainingAccounts undefined");
    if (!transaction.remainingAccounts.nullifierPdaPubkeys)
      throw new Error("remainingAccounts.nullifierPdaPubkeys undefined");
    if (!transaction.remainingAccounts.leavesPdaPubkeys)
      throw new Error("remainingAccounts.leavesPdaPubkeys undefined");
    if (!transaction.transactionInputs.publicInputs)
      throw new Error("params.publicInputs undefined");
    if (!transaction.params.relayer)
      throw new Error("params.params.relayer undefined");
    if (!transaction.params.encryptedUtxos)
      throw new Error("params.encryptedUtxos undefined");
    if (!this.verifierProgram) throw new Error("verifierProgram undefined");
    // TODO: check if this is still required
    if (
      !transaction.provider.browserWallet &&
      !transaction.provider.nodeWallet
    ) {
      throw new Error("Payer(browserwallet, nodewallet) not defined");
    }
    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        transaction.transactionInputs.publicInputs.publicAmount,
        transaction.transactionInputs.publicInputs.nullifiers,
        transaction.transactionInputs.publicInputs.leaves[0],
        transaction.transactionInputs.publicInputs.feeAmount,
        new anchor.BN(transaction.transactionInputs.rootIndex.toString()),
        new anchor.BN(transaction.params.relayer.relayerFee.toString()),
        Buffer.from(transaction.params.encryptedUtxos),
      )
      .accounts({
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
      })
      .instruction();

    const ix2 = await this.verifierProgram.methods
      .shieldedTransferSecond(
        transaction.transactionInputs.proofBytes.proofA,
        transaction.transactionInputs.proofBytes.proofB,
        transaction.transactionInputs.proofBytes.proofC,
      )
      .accounts({
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
      })
      .remainingAccounts([
        ...transaction.remainingAccounts.nullifierPdaPubkeys,
        ...transaction.remainingAccounts.leavesPdaPubkeys,
      ])
      .instruction();
    this.instructions = [ix1, ix2];
    return this.instructions;
  }
}
