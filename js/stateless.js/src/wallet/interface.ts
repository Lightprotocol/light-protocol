/// TODO: extract wallet into its own npm package
import {
  Commitment,
  Connection,
  Keypair,
  VersionedTransaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { PublicKey, Transaction } from "@solana/web3.js";
import { sign } from "tweetnacl";
import { Proof, getProofInternal } from "./get-proof";
import { Idl } from "@coral-xyz/anchor";
import { IDL } from "../idls/psp_compressed_pda";

export type InclusionProofPublicInputs = {
  root: string;
  leaf: string;
};
export type InclusionProofPrivateInputs = {
  merkleProof: string[];
  leaf: string;
  leafIndex: string;
};

/// On the system level, we're proving simple inclusion proofs in a
/// state tree, for each utxo used as input into a transaction.
export type InclusionProofInputs = (InclusionProofPublicInputs &
  InclusionProofPrivateInputs)[];

/// Mock Solana web3 library
export class Wallet {
  _publicKey: PublicKey;
  _keypair: Keypair;
  _connection: Connection;
  _url: string;
  _commitment: Commitment;

  constructor(keypair: Keypair, url: string, commitment: Commitment) {
    this._publicKey = keypair.publicKey;
    this._keypair = keypair;
    this._connection = new Connection(url);
    this._url = url;
    this._commitment = commitment;
  }

  signTransaction = async (tx: any): Promise<any> => {
    await tx.sign([this._keypair!]);
    return tx;
  };

  sendTransaction = async (
    transaction: VersionedTransaction
  ): Promise<string> => {
    const signature = await this._connection.sendTransaction(transaction);
    return signature;
  };

  signAllTransactions = async <T extends Transaction | VersionedTransaction>(
    transactions: T[]
  ): Promise<T[]> => {
    const signedTxs = await Promise.all(
      transactions.map(async (tx) => {
        return await this.signTransaction(tx);
      })
    );
    return signedTxs;
  };

  signMessage = async (message: Uint8Array): Promise<Uint8Array> => {
    return sign.detached(message, this._keypair.secretKey);
  };

  sendAndConfirmTransaction = async (
    transaction: Transaction,
    signers = []
  ): Promise<any> => {
    const response = await sendAndConfirmTransaction(
      this._connection,
      transaction,
      [this._keypair, ...signers],
      {
        commitment: this._commitment,
      }
    );
    return response;
  };

  getProof = async (proofInputs: InclusionProofInputs): Promise<Proof> => {
    /// should pick verifer idl and circuit here. Eventually, we can consider
    /// adding a parameter for dapps to request custom circuits by idl. Though,
    /// this is mostly contingent on whether we want to support custom
    /// circuits in the wallet registry or keep them in the dapp.
    const _verifierIdl = getIdlByProofInputs(proofInputs);
    const _circuitName = getCircuitByProofInputs(proofInputs);

    // const { parsedProof, parsedPublicInputsObject } = await getProofInternal({
    //   /// TODO: implement actual path
    //   firstPath: "mockPath",
    //   verifierIdl,
    //   circuitName,
    //   proofInputs,
    //   enableLogging: true,
    //   verify: true,
    // });

    return {
      parsedProof: "mockParsedProof",
      parsedPublicInputsObject: "mockParsedPublicInputsObject",
    };
  };
}
/// TODO: generalize when needed
const getIdlByProofInputs = (_proofInputs: InclusionProofInputs): Idl => {
  return IDL;
};

/// TODO: use actual circuits
/// Picks the circuit by amount of proof inputs
const getCircuitByProofInputs = (
  _proofInputs: InclusionProofInputs
): string => {
  return "mockCircuit";
};
