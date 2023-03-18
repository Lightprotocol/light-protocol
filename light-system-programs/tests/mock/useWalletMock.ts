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

  constructor(keypair: Keypair) {
    this._publicKey = keypair.publicKey;
    this._keypair = keypair;
  }

   signTransaction = async (tx) => {
    await tx.sign([this._keypair!]);
    return tx;
  };

  signMessage = async (message) => {
    return sign.detached(message, this._keypair.secretKey);
  };

  sendAndConfirmTransaction = async (fn) => {
    return await fn();
  };

}

// Mock useWallet hook
export const useWallet = (wallet: Keypair) => {
  const provider = new MockProvider(wallet);
  return {
    publicKey: provider._publicKey,
    signMessage: async (message): Promise<Uint8Array> => {
      return await provider.signTransaction(message);
    },
    signTransaction: async (transaction): Promise<any> => {
      return await provider.signTransaction(transaction);
    },
    sendAndConfirmTransaction: async (transactions) =>
      provider.sendAndConfirmTransaction(transactions),
  };
};
