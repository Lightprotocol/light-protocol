import { VerifierProgramOne, IDL_VERIFIER_PROGRAM_ONE } from "../idls/index";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import {
  hashAndTruncateToCircuit,
  TransactionErrorCode,
  VerifierError,
  VerifierErrorCode,
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
    if (!transaction.params)
      throw new VerifierError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getInstructions",
      );
    if (!transaction.remainingAccounts)
      throw new VerifierError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getInstructions",
        "verifier one: remainingAccounts undefined",
      );
    if (!transaction.remainingAccounts.nullifierPdaPubkeys)
      throw new VerifierError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getInstructions",
        "verifier one: remainingAccounts.nullifierPdaPubkeys undefined",
      );
    if (!transaction.remainingAccounts.leavesPdaPubkeys)
      throw new VerifierError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getInstructions",
        "verifier one: remainingAccounts.leavesPdaPubkeys undefined",
      );
    if (!transaction.transactionInputs.publicInputs)
      throw new VerifierError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getInstructions",
        "verifier one: params.publicInputs undefined",
      );
    if (!transaction.params.relayer)
      throw new VerifierError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getInstructions",
        "verifier one: params.params.relayer undefined",
      );
    if (!transaction.params.encryptedUtxos)
      throw new VerifierError(
        VerifierErrorCode.ENCRYPTING_UTXOS_UNDEFINED,
        "getInstructions",
        "verifier one: params.encryptedUtxos undefined",
      );
    if (!transaction.provider.wallet) {
      throw new VerifierError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "getInstructions",
        "verifier one: Payer(wallet) not defined",
      );
    }
    if (!this.verifierProgram)
      throw new VerifierError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getInstructions",
        "verifier one: verifierProgram undefined",
      );

    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        transaction.transactionInputs.publicInputs.publicAmount,
        transaction.transactionInputs.publicInputs.nullifiers,
        transaction.transactionInputs.publicInputs.leaves[0],
        transaction.transactionInputs.publicInputs.feeAmount,
        new anchor.BN(transaction.transactionInputs.rootIndex.toString()),
        new anchor.BN(
          transaction.params.relayer
            .getRelayerFee(transaction.params.ataCreationFee)
            .toString(),
        ),
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
