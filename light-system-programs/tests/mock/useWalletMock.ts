import {
  Connection,
  Keypair,
  sendAndConfirmTransaction,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import nacl from "tweetnacl";

import { sign } from "tweetnacl";

// Mock Solana web3 library
class MockProvider {
  _publicKey: PublicKey;
  _keypair: Keypair;
  _connection: Connection;

  constructor(keypair: Keypair, connection: Connection) {
    this._publicKey = keypair.publicKey;
    this._keypair = keypair;
    this._connection = connection;
  }

  signTransaction = async (transaction) => {
    await transaction.sign([this._keypair!]);
    return transaction
  };

  async sendTransaction(transaction) {
    // Simulate transaction submission
    console.log("Mock transaction submitted:", transaction);
    return "mockTransactionSignature";
  }

  async signAllTransactions(transactions) {
    // Simulate signing all transactions
    console.log("Mock transactions signed:", transactions);
    return transactions;
  }

  async sendAndConfirmTransaction(transaction) {
    try {
      console.log("are we using this one? ??transaction here ============>", { transaction });
      const signature = await transaction.sign(this._keypair.secretKey);
      return await sendAndConfirmTransaction(
        this._connection,
        transaction,
        [signature],
        {
          commitment: "singleGossip",
          preflightCommitment: "singleGossip",
        },
      );
    } catch (err) {}
  }


  signMessage(message: Uint8Array): Promise<Uint8Array> {
    return new Promise(async (resolve, reject) => {
      try {
        const signature = nacl.sign.detached(message, this._keypair.secretKey);
        return resolve(signature);
      } catch (err) {
        console.log({ err });
        reject(err);
      }
    });
  }

  async connect() {
    // Simulate wallet connection
    console.log("Mock wallet connected");
  }

  async disconnect() {
    // Simulate wallet disconnection
    console.log("Mock wallet disconnected");
  }
}

// Mock useWallet hook
export const useWallet = (wallet: Keypair, connection: Connection) => {
  const provider = new MockProvider(wallet, connection);
  return {
    publicKey: provider._publicKey,
    connect: async () => provider.connect(),
    disconnect: async () => provider.disconnect(),
    signMessage: async (message): Promise<Uint8Array> => {
      return await provider.signTransaction(message);
    },
    signTransaction: async (transaction): Promise<any> => {
      return await provider.signTransaction(transaction);
    },
    signAllTransactions: async (transactions) =>
      provider.signAllTransactions(transactions),
    sendAndConfirmTransaction: async (transactions) =>
      provider.signAllTransactions(transactions),
  };
};
