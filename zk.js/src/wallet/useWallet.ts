import {
  Commitment,
  Connection,
  Keypair,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { PublicKey, Transaction } from "@solana/web3.js";
import { sign } from "tweetnacl";
import { Provider, Wallet as WrappedWallet } from "./provider";
import { Balance } from "../types/balance";
import { Account, Relayer } from "index";
import { syncBalance } from "balance/balance";
import { WasmHasher } from "@lightprotocol/account.rs";

// Mock Solana-Wallet-Adapter Wallet interface
// Plus our own wrappers that we need in the wallet adapter
class Wallet {
  _publicKey: PublicKey;
  _keypair: Keypair;
  _connection: Connection;
  _url: string;
  _commitment: Commitment;
  /** Our own extension */
  private _compressedBalance: Balance | undefined;
  private _account: Account | undefined;

  constructor(keypair: Keypair, url: string, commitment: Commitment) {
    this._publicKey = keypair.publicKey;
    this._keypair = keypair;
    this._connection = new Connection(url);
    this._url = url;
    this._commitment = commitment;
    this._compressedBalance = undefined; /// needs to be inited manually (at first ivocation of getCompressedBalance)
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

  /** This mocks the interface, the wallet would implement this themselves inside their wallet */
  getProof = async (tx: Transaction): Promise<any> => {};

  /** Decrypt a batch of UTXOs (byte arrays) */
  decryptState = async (encryptedState: Uint8Array): Promise<any> => {};
  getAssetBalances = async (): Promise<any> => {
    /// wallets should cache/persist this
    const hasher = await WasmHasher.getInstance();
    /// wallets should cache/persist this
    const relayer = await Relayer.initFromUrl("https://helius.xyz/...");

    const account = await this.getAccount();

    const assetLookupTable = [];

    /**
     * The wallet periodically sync it's balance with the latest merkletree state
     * Ideally, RPC providers will expose getAssetsByOwner
     */
    const balance = await syncBalance({
      connection: this._connection,
      relayer,
      account,
      hasher,
      assetLookupTable,
      balance: this._compressedBalance ?? undefined,
      _until: undefined,
    });
    this._compressedBalance = balance;
  };

  private getAccount = async (): Promise<Account> => {
    if (!this._account) {
      //@ts-ignore
      this._account = 1;
      return this._account!;
    }
    return this._account;
  };
}

// Mock useWallet hook
export const useWallet = (
  keypair: Keypair,
  url: string = "http://127.0.0.1:8899",
  isNodeWallet: boolean = true,
  commitment: Commitment = "confirmed",
): WrappedWallet => {
  url = url !== "mock" ? url : "http://127.0.0.1:8899";
  const wallet = new Wallet(keypair, url, commitment);
  return {
    publicKey: wallet._publicKey,
    sendAndConfirmTransaction: wallet.sendAndConfirmTransaction,
    signMessage: wallet.signMessage,
    signTransaction: wallet.signTransaction,
    signAllTransactions: wallet.signAllTransactions,
    isNodeWallet,
    getProof: wallet.getProof,
    getAssetBalances: wallet.getAssetBalances,
    decryptState: wallet.decryptState,
  };
};

/// Type guard function
export function isSolanaKeypair(obj: Keypair | WrappedWallet): obj is Keypair {
  if ("secretKey" in obj) return true;
  else return false;
}
