import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  hashAndTruncateToCircuit,
  Provider,
  TransactionErrorCode,
  VerifierError,
  VerifierErrorCode,
  verifierProgramZeroProgramId,
} from "../index";
import { Transaction } from "transaction";
import { Verifier, PublicInputs, VerifierConfig } from ".";
import { VerifierProgramZero, IDL_VERIFIER_PROGRAM_ZERO } from "../idls/index";

// TODO: define verifier with an Idl thus absorb this functionality into the Transaction class
export class VerifierZero implements Verifier {
  verifierProgram?: Program<VerifierProgramZero>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: VerifierConfig;
  instructions?: anchor.web3.TransactionInstruction[];
  pubkey: anchor.BN;
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
      inputNullifier: [publicInputsBytes[5], publicInputsBytes[6]],
      outputCommitment: [publicInputsBytes[7], publicInputsBytes[8]],
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
        "verifier zero: remainingAccounts undefined",
      );
    if (!transaction.remainingAccounts.nullifierPdaPubkeys)
      throw new VerifierError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getInstructions",
        "verifier zero: remainingAccounts.nullifierPdaPubkeys undefined",
      );
    if (!transaction.remainingAccounts.leavesPdaPubkeys)
      throw new VerifierError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getInstructions",
        "verifier zero: remainingAccounts.leavesPdaPubkeys undefined",
      );
    if (!transaction.transactionInputs.publicInputs)
      throw new VerifierError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getInstructions",
        "verifier zero: params.publicInputs undefined",
      );
    if (!transaction.params.relayer)
      throw new VerifierError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getInstructions",
        "verifier zero: params.params.relayer undefined",
      );
    if (!transaction.params.encryptedUtxos)
      throw new VerifierError(
        VerifierErrorCode.ENCRYPTING_UTXOS_UNDEFINED,
        "getInstructions",
        "verifier zero: params.encryptedUtxos undefined",
      );
    if (!transaction.provider.wallet) {
      throw new VerifierError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "getInstructions",
        "verifier zero: Payer(wallet) not defined",
      );
    }
    if (!this.verifierProgram)
      throw new VerifierError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getInstructions",
        "verifier zero: verifierProgram undefined",
      );

    const ix = await this.verifierProgram.methods
      .shieldedTransferInputs(
        transaction.transactionInputs.proofBytes.proofA,
        transaction.transactionInputs.proofBytes.proofB,
        transaction.transactionInputs.proofBytes.proofC,
        transaction.transactionInputs.publicInputs.publicAmountSpl,
        transaction.transactionInputs.publicInputs.inputNullifier,
        transaction.transactionInputs.publicInputs.outputCommitment,
        transaction.transactionInputs.publicInputs.publicAmountSol,
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
      .remainingAccounts([
        ...transaction.remainingAccounts.nullifierPdaPubkeys,
        ...transaction.remainingAccounts.leavesPdaPubkeys,
      ])
      .instruction();
    this.instructions = [ix];
    return [ix];
  }
}
