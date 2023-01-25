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
  transaction: Transaction;
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
    this.wtnsGenPath = "appTransaction_js/appTransaction.wasm";
    this.zkeyPath = "appTransaction.zkey";
    this.calculateWtns = require("../build-circuit/appTransaction_js/witness_calculator.js");
    // ../build-circuits/transactionApp_js/witness_calculator.js
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
        connectingHash: publicInputsBytes[1],
        verifier: publicInputsBytes[0],
      };
    } else {
      throw new Error(`publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.nrPublicInputs}`);
    }
  }

  // test transferFirst
  // read bytes from verifierState if already exists and refetch getPdaAddresses();

  // TODO: discuss with Swen how to split this into send and confirm,
  async getInstructions(transaction: Transaction): Promise<TransactionInstruction[]> {

    const invokingVerifierPubkey = (
      await PublicKey.findProgramAddress(
        [
          transaction.payer.publicKey.toBytes(),
          // anchor.utils.bytes.utf8.encode("VERIFIER_STATE"),
        ],
        this.verifierProgram.programId
      ))[0];
    // await transaction.instance.provider.connection.confirmTransaction(
    //   await transaction.instance.provider.connection.requestAirdrop(invokingVerifierPubkey, 1_000_000_000, "confirmed")
    // );
    await transaction.instance.provider.connection.confirmTransaction(
      await transaction.instance.provider.connection.requestAirdrop(transaction.params.accounts.authority, 1_000_000_000, "confirmed")
    );
    
    console.log("pre ix1");
    console.log("transaction.publicInputs ", transaction.publicInputs);
    
    console.log("new BN(transaction.publicInputs.publicAmount) ", transaction.publicInputs.publicAmount);
    console.log("ntransaction.publicInputs.nullifiers ", transaction.publicInputs.nullifiers);
    console.log("transaction.publicInputs.leaves ", transaction.publicInputs.leaves);
    console.log("new BN(transaction.publicInputs.feeAmount) ",Buffer.from(transaction.publicInputs.feeAmount));
    console.log("new anchor.BN(transaction.rootIndex.toString()) ", new anchor.BN(transaction.rootIndex.toString()));
    console.log("new anchor.BN(transaction.relayer.relayerFee.toString()) ", new anchor.BN(transaction.relayer.relayerFee.toString()));
    console.log("transaction.encryptedUtxos ", transaction.encryptedUtxos.length);
    console.log(transaction.appParams);
    console.log("transaction.publicInputsApp ", transaction.publicInputsApp);
    console.log("transaction.appParams.input ", transaction.appParams);
    console.log("transaction.params.accounts ", transaction.params.accounts);
    
    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves,
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()), // could make this smaller to u16
        new anchor.BN(transaction.relayer.relayerFee.toString()),
        Buffer.from(transaction.encryptedUtxos.slice(0, 512)),
        // transaction.publicInputsApp.slot.slice(24,32)//.reverse()
        // got 23 bytes left
      )
      .accounts({
        ...transaction.params.accounts,
      })
      .instruction();
      console.log("pre ix2");
      // console.log("transaction.publicInputsApp.connectingHash ", transaction.publicInputsApp.connectingHash);
      
    const ix2 = await this.verifierProgram.methods
      .shieldedTransferSecond(
        Buffer.from(transaction.proofBytesApp),
        Buffer.from(transaction.proofBytes),
        Buffer.from(transaction.publicInputsApp.connectingHash)
      )
      .accounts({
        verifierProgram: transaction.params.verifier.verifierProgram.programId,
        invokingVerifier: invokingVerifierPubkey,
        ...transaction.params.accounts,
        ...transaction.relayer.accounts,
      })
      .remainingAccounts([
        ...transaction.params.nullifierPdaPubkeys,
        ...transaction.params.leavesPdaPubkeys,
      ])
      .signers([transaction.payer])
      .instruction();

    return [ix1, ix2];
  }
}
