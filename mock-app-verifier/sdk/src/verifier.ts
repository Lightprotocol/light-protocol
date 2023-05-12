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
import { BN, Idl } from "@coral-xyz/anchor";
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
  proofBytes: any;
  messageDataLength: number;
  fetchedOfferUtxos: Utxo[];
  config: { in: number; out: number; isAppVerifier: boolean };
  pubkey: BN;
  idl: Idl;
  constructor() {
    this.config = { in: 4, out: 4, isAppVerifier: true };
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
    this.idl = IDL;
  }
  // publicInputsBytes: Uint8Array
  parsePublicInputsFromArray(publicInputsBytes: Uint8Array): PublicInputs {
    if (publicInputsBytes.length == this.nrPublicInputs) {
      return {
        publicAppVerifier: publicInputsBytes[0],
        transactionHash: publicInputsBytes[1],
      };
    } else {
      throw new Error(
        `publicInputsBytes.length invalid ${publicInputsBytes.length} != ${this.nrPublicInputs}`,
      );
    }
  }
}
