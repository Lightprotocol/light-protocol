import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
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
  Transaction,
  PublicInputs,
} from "light-sdk";

import { marketPlaceVerifierProgramId } from "./constants";
import { BN } from "@project-serum/anchor";
import {
  IDL,
  MockVerifier as MockVerifierType,
} from "../../target/types/mock_verifier";

export class MockVerifier implements Verifier {
  verifierProgram?: Program<MockVerifierType>;
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
    this.verifierProgram = new Program(IDL, marketPlaceVerifierProgramId);
    this.instructions = [];
    this.wtnsGenPath = "appTransaction_js/appTransaction.wasm";
    this.zkeyPath = "appTransaction.zkey";
    this.calculateWtns = require("../build-circuit/appTransaction_js/witness_calculator.js");
    // ../build-circuits/transactionApp_js/witness_calculator.js
    this.nrPublicInputs = 2;
    // TODO: implement check that encryptedUtxos.length == this.messageDataLength
    this.messageDataLength = 512;
    this.pubkey = hashAndTruncateToCircuit(
      this.verifierProgram.programId.toBytes(),
    );
  }

  parsePublicInputsFromArray(publicInputsBytes: Uint8Array): PublicInputs {
    if (publicInputsBytes.length == this.nrPublicInputs) {
      return {
        transactionHash: publicInputsBytes[1],
        publicAppVerifier: publicInputsBytes[0],
      };
    } else {
      throw new Error(
        `publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.nrPublicInputs}`,
      );
    }
  }

  // test transferFirst
  // read bytes from verifierState if already exists and refetch getPdaAddresses();

  // TODO: discuss with Swen how to split this into send and confirm,
  async getInstructions(
    transaction: Transaction,
  ): Promise<TransactionInstruction[]> {
    const invokingVerifierPubkey = (
      await PublicKey.findProgramAddress(
        [
          transaction.provider.wallet.publicKey.toBytes()
          // anchor.utils.bytes.utf8.encode("VERIFIER_STATE"),
        ],
        this.verifierProgram.programId,
      )
    )[0];
    // await transaction.instance.provider.connection.confirmTransaction(
    //   await transaction.instance.provider.connection.requestAirdrop(invokingVerifierPubkey, 1_000_000_000, "confirmed")
    // );

    // console.log("pre ix1");
    // console.log("transaction.publicInputs ", transaction.publicInputs);

    // console.log("new BN(transaction.publicInputs.publicAmountSpl) ", transaction.publicInputs.publicAmountSpl);
    // console.log("ntransaction.publicInputs.nullifiers ", transaction.publicInputs.nullifiers);
    // console.log("transaction.publicInputs.leaves ", transaction.publicInputs.leaves);
    // console.log("new BN(transaction.publicInputs.publicAmountSol) ",Buffer.from(transaction.publicInputs.publicAmountSol));
    // console.log("new anchor.BN(transaction.rootIndex.toString()) ", new anchor.BN(transaction.rootIndex.toString()));
    // console.log("new anchor.BN(transaction.params.relayer.relayerFee.toString()) ", new anchor.BN(transaction.relayer.relayerFee.toString()));
    // console.log("transaction.encryptedUtxos ", transaction.encryptedUtxos.length);
    // console.log(transaction.appParams);
    // console.log("transaction.publicInputsApp ", transaction.publicInputsApp);
    // console.log("transaction.appParams.input ", transaction.appParams);
    // console.log("transaction.params.accounts ", transaction.params.accounts);

    var relayerRecipientSol = transaction.params.relayer.accounts.relayerRecipientSol;

    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        transaction.transactionInputs.publicInputs.publicAmountSpl,
        transaction.transactionInputs.publicInputs.nullifiers,
        transaction.transactionInputs.publicInputs.leaves,
        transaction.transactionInputs.publicInputs.publicAmountSol,
        new anchor.BN(transaction.transactionInputs.rootIndex.toString()), // could make this smaller to u16
        new anchor.BN(transaction.params.relayer.relayerFee.toString()),
        Buffer.from(transaction.params.encryptedUtxos.slice(0, 512)),
      )
      .accounts({
        ...transaction.params.accounts,
      })
      .instruction();
    // console.log("pre ix2");
    // console.log("transaction.publicInputsApp.connectingHash ", transaction.publicInputsApp.connectingHash);

    const ix2 = await this.verifierProgram.methods
      .shieldedTransferSecond(
        transaction.transactionInputs.proofBytesApp.proofA,
        transaction.transactionInputs.proofBytesApp.proofB,
        transaction.transactionInputs.proofBytesApp.proofC,
        transaction.transactionInputs.proofBytes.proofA,
        transaction.transactionInputs.proofBytes.proofB,
        transaction.transactionInputs.proofBytes.proofC,
        Buffer.from(transaction.transactionInputs.publicInputsApp.transactionHash)
      )
      .accounts({
        verifierProgram: transaction.params.verifier.verifierProgram.programId,
        ...transaction.params.accounts,
        ...transaction.params.relayer.accounts,
        relayerRecipientSol: relayerRecipientSol,
      })
      .remainingAccounts([
        ...transaction.remainingAccounts.nullifierPdaPubkeys,
        ...transaction.remainingAccounts.leavesPdaPubkeys,
      ])
      .instruction();

    this.instructions.push(ix1);
    this.instructions.push(ix2);
    return this.instructions;
  }
}
