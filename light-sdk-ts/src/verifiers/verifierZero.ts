import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  hashAndTruncateToCircuit,
  verifierProgramZeroProgramId,
} from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { VerifierProgramZero, IDL_VERIFIER_PROGRAM_ZERO } from "../idls/index";

// TODO: define verifier with an Idl thus absorb this functionality into the Transaction class
export class VerifierZero implements Verifier {
  verifierProgram: Program<VerifierProgramZero>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  config: { in: number; out: number };
  instructions?: anchor.web3.TransactionInstruction[];
  pubkey: anchor.BN;
  constructor() {
    try {
      this.verifierProgram = new Program(
        IDL_VERIFIER_PROGRAM_ZERO,
        verifierProgramZeroProgramId,
      );
    } catch (error) {
      console.log(error);
    }
    // ./build-circuits/transactionMasp2_js/
    this.wtnsGenPath = "transactionMasp2_js/transactionMasp2.wasm";
    this.zkeyPath = `transactionMasp2.zkey`;
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.config = { in: 2, out: 2 };
    this.pubkey = hashAndTruncateToCircuit(
      verifierProgramZeroProgramId.toBytes(),
    );
  }

  parsePublicInputsFromArray(publicInputsBytes: Uint8Array): PublicInputs {
    if (publicInputsBytes) {
      if (publicInputsBytes.length == 9) {
        return {
          root: publicInputsBytes[0],
          publicAmount: publicInputsBytes[1],
          extDataHash: publicInputsBytes[2],
          feeAmount: publicInputsBytes[3],
          mintPubkey: publicInputsBytes[4],
          nullifiers: [publicInputsBytes[5], publicInputsBytes[6]],
          leaves: [[publicInputsBytes[7], publicInputsBytes[8]]],
        };
      } else {
        throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != 9`;
      }
    } else {
      throw new Error("public input bytes undefined");
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
    if (
      !transaction.provider.browserWallet &&
      !transaction.provider.nodeWallet
    ) {
      throw new Error("Payer(browserwallet, nodewallet) not defined");
    }

    const ix = await this.verifierProgram.methods
      .shieldedTransferInputs(
        Buffer.from(transaction.proofBytes),
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves[0],
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()),
        new anchor.BN(transaction.params.relayer.relayerFee.toString()),
        Buffer.from(transaction.params.encryptedUtxos.slice(0, 190)), // remaining bytes can be used once tx sizes increase
      )
      .accounts({
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
      })
      .remainingAccounts([
        ...transaction.params.nullifierPdaPubkeys,
        ...transaction.params.leavesPdaPubkeys,
      ])
      .instruction();
    this.instructions = [ix];
    return [ix];
  }
}
