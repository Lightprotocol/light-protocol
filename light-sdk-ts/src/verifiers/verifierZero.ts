import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  hashAndTruncateToCircuit,
  verifierProgramZeroProgramId,
} from "../index";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import {
  VerifierProgramZero,
  VerifierProgramZeroIdl,
} from "../idls/verifier_program_zero";
import { PublicKey } from "@solana/web3.js";
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
  pubkey: anchor.BN;
  constructor() {
    try {
      this.verifierProgram = new Program(
        VerifierProgramZero,
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
          Buffer.from(transaction.encryptedUtxos.slice(0, 190)), // remaining bytes can be used once tx sizes increase
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
        "transaction.params, nullifierPdaPubkeys or leavesPdaPubkeys undefined",
      );
    }
  }
}
