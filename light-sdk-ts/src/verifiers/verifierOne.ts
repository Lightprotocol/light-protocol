import { VerifierProgramOne, IDL_VERIFIER_PROGRAM_ONE } from "../idls/index";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import {
  hashAndTruncateToCircuit,
  verifierProgramOneProgramId,
} from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";

export class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOne>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number };
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
    this.config = { in: 10, out: 2 };
    this.pubkey = hashAndTruncateToCircuit(
      verifierProgramOneProgramId.toBytes(),
    );
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
    if (!transaction.params) throw new Error("params undefined");
    if (!transaction.params.nullifierPdaPubkeys)
      throw new Error("params.nullifierPdaPubkeys undefined");
    if (!transaction.params.leavesPdaPubkeys)
      throw new Error("params.leavesPdaPubkeys undefined");
    if (!transaction.publicInputs)
      throw new Error("params.publicInputs undefined");
    if (!transaction.params.relayer)
      throw new Error("params.params.relayer undefined");
    if (!transaction.params.encryptedUtxos)
      throw new Error("params.encryptedUtxos undefined");

    // TODO: check if this is still required
    if (
      !transaction.provider.browserWallet &&
      !transaction.provider.nodeWallet
    ) {
      throw new Error("Payer(browserwallet, nodewallet) not defined");
    }
    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves[0],
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()),
        new anchor.BN(transaction.params.relayer.relayerFee.toString()),
        Buffer.from(transaction.params.encryptedUtxos),
      )
      .accounts({
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
      })
      .instruction();

    const ix2 = await this.verifierProgram.methods
      .shieldedTransferSecond(Buffer.from(transaction.proofBytes))
      .accounts({
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
      })
      .remainingAccounts([
        ...transaction.params.nullifierPdaPubkeys,
        ...transaction.params.leavesPdaPubkeys,
      ])
      .instruction();
    this.instructions = [ix1, ix2];
    return this.instructions;
  }
}
