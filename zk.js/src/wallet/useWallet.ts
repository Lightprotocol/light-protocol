import {
  Commitment,
  Connection,
  Keypair,
  VersionedTransaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { PublicKey, Transaction } from "@solana/web3.js";
import { sign } from "tweetnacl";

// Mock Solana web3 library
class Wallet {
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

  signAllTransactions = async (
    transactions: Transaction[],
  ): Promise<Transaction[]> => {
    const signedTxs = await Promise.all(
      transactions.map(async (tx) => {
        return await this.signTransaction(tx);
      }),
    );
    return signedTxs;
  };

  signMessage = async (message: Uint8Array): Promise<Uint8Array> => {
    return sign.detached(message, this._keypair.secretKey);
  };

  sendTransaction = async (transaction: VersionedTransaction): Promise<any> => {
    const response = await this._connection.sendTransaction(transaction);
    return response;
  };

  sendAndConfirmTransaction = async (
    transaction: Transaction,
    signers = [],
  ): Promise<any> => {
    const response = await sendAndConfirmTransaction(
      this._connection,
      transaction,
      [this._keypair, ...signers],
      {
        commitment: this._commitment,
      },
    );
    return response;
  };
}

// Mock useWallet hook
export const useWallet = (
  keypair: Keypair,
  url: string = "http://127.0.0.1:8899",
  isNodeWallet: boolean = true,
  commitment: Commitment = "confirmed",
) => {
  url = url !== "mock" ? url : "http://127.0.0.1:8899";
  const wallet = new Wallet(keypair, url, commitment);
  return {
    publicKey: wallet._publicKey,
    sendTransaction: wallet.sendTransaction,
    sendAndConfirmTransaction: wallet.sendAndConfirmTransaction,
    signMessage: wallet.signMessage,
    signTransaction: wallet.signTransaction,
    signAllTransactions: wallet.signAllTransactions,
    isNodeWallet,
  };
};
