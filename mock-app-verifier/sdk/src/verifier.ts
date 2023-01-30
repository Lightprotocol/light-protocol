import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {
  Transaction as SolanaTransaction,
  PublicKey,
  Keypair as SolanaKeypair,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  Verifier,
  Utxo,
  hashAndTruncateToCircuit,
  Transaction
} from "light-sdk";

import {
  marketPlaceVerifierProgramId,
} from "./constants";
import { BN } from "@project-serum/anchor";
import { IDL, MockVerifier as MockVerifierType } from "../../target/types/mock_verifier";
import { assert } from "chai";

export class MockVerifier implements Verifier {
  verifierProgram: Program<MockVerifierType>;
  verifierProgramIdCircuit: BN;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  nrPublicInputs: number;
  instructions?: anchor.web3.TransactionInstruction[];
  verifierStatePubkey: PublicKey;
  proofBytes: Uint8Array;
  messageDataLength: number;
  fetchedOfferUtxos: Utxo[];
  config: { in: number; out: number; app?: boolean };
  pubkey: BN;
  constructor() {
    this.config = { in: 4, out: 4, app: true };
    this.verifierProgram = new Program(
      IDL,
      marketPlaceVerifierProgramId
    );
    this.instructions = []
    this.wtnsGenPath = "appTransaction_js/appTransaction.wasm";
    this.zkeyPath = "appTransaction.zkey";
    this.calculateWtns = require("../build-circuit/appTransaction_js/witness_calculator.js");
    this.nrPublicInputs = 2;
    // TODO: implement check that encryptedUtxos.length == this.messageDataLength
    this.messageDataLength = 512;
    this.pubkey = hashAndTruncateToCircuit(
      this.verifierProgram.programId.toBytes()
    );
  }

  parsePublicInputsFromArray(publicInputsBytes: Uint8Array) {

    if (publicInputsBytes.length == this.nrPublicInputs) {

      return {
        verifier: publicInputsBytes[0],
        connectingHash: publicInputsBytes[1],
      };
    } else {
      throw new Error(`publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.nrPublicInputs}`);
    }
  }

  // test transferFirst
  // read bytes from verifierState if already exists and refetch getPdaAddresses();

  // TODO: discuss with Swen how to split this into send and confirm,
  async getInstructions(transaction: Transaction): Promise<TransactionInstruction[]> {
    
    console.log("pre ix1");
 
    var relayerRecipient = transaction.relayer.accounts.relayerRecipient;
    try {
      // deposit means the amount is u64
      new BN(transaction.publicInputs.feeAmount).toArray("be", 8);    
      relayerRecipient = transaction.params.accounts.escrow;      
    } catch (error) {   }
    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves,
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()), // could make this smaller to u16
        new anchor.BN(transaction.relayer.relayerFee.toString()),
        Buffer.from(transaction.encryptedUtxos.slice(0, 512)),
      )
      .accounts({
        ...transaction.params.accounts,
      })
      .instruction();
      console.log("pre ix2");
    
    const ix2 = await this.verifierProgram.methods
      .shieldedTransferSecond(
        Buffer.from(transaction.proofBytesApp),
        Buffer.from(transaction.proofBytes),
        Buffer.from(transaction.publicInputsApp.connectingHash)
      )
      .accounts({
        verifierProgram: transaction.params.verifier.verifierProgram.programId,
        ...transaction.params.accounts,
        ...transaction.relayer.accounts,
        relayerRecipient: relayerRecipient,
      })
      .remainingAccounts([
        ...transaction.params.nullifierPdaPubkeys,
        ...transaction.params.leavesPdaPubkeys,
      ])
      .signers([transaction.payer])
      .instruction();
    
    this.instructions.push(ix1)
    this.instructions.push(ix2)
    return this.instructions;
  }
}
