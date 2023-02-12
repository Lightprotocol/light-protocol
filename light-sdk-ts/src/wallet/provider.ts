import { AnchorError, AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { SolMerkleTree } from "@merkleTree";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
} from "@solana/web3.js";
import { initLookUpTable, initLookUpTableFromFile } from "test-utils";
const circomlibjs = require("circomlibjs");

/**
 * use: signMessage, signTransaction, sendAndConfirmTransaction, publicKey from the useWallet() hook in solana/wallet-adapter and {connection} from useConnection()
 */
export type BrowserWallet = {
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  sendAndConfirmTransaction: (transaction: any) => Promise<any>;
  publicKey: PublicKey;
};

/**
 * Provides: payer/wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add relayer here; default deriv, if passed in can choose custom relayer.
export class Provider {
  connection?: Connection;
  browserWallet?: BrowserWallet;
  nodeWallet?: SolanaKeypair;
  payer?: SolanaKeypair;
  confirmConfig: ConfirmOptions;
  poseidon: any;
  lookUpTable?: PublicKey;
  solMerkleTree?: SolMerkleTree;
  provider?: AnchorProvider; // temp -?
  url?: string;

  /**
   * Init either with nodeWallet or browserWallet. Default feepayer is the provided wallet, optionally override with payer.
   * Optionally provide confirmConfig, Default = 'confirmed'.
   */
  constructor({
    nodeWallet,
    browserWallet,
    payer,
    confirmConfig,
    connection,
    url = "http://127.0.0.1:8899",
  }: {
    nodeWallet?: SolanaKeypair;
    browserWallet?: BrowserWallet;
    payer?: SolanaKeypair;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url?: string;
  }) {
    if (nodeWallet && browserWallet)
      throw new Error("Both node and browser environments provided.");
    if (!nodeWallet && !browserWallet) throw new Error("No wallet provided.");
    if (nodeWallet && !url) throw new Error("No url provided.");
    if (browserWallet && !connection)
      throw new Error("No connection provided.");
    if (nodeWallet && connection)
      throw new Error(
        "Connection provided in node environment. Provide a url instead",
      );
    if (browserWallet && url)
      throw new Error(
        "Url provided in browser environment. Provide a connection instead",
      );
    if (payer)
      throw new Error("Custom feepayer override is not yet supported.");

    this.confirmConfig = confirmConfig || { commitment: "confirmed" };

    if (nodeWallet) {
      this.nodeWallet = nodeWallet;
      // TODO: check if we can remove this.url!
      this.url = url;
      // TODO: check if we can remove this.provider!
      setProvider(AnchorProvider.env());
      this.provider = AnchorProvider.local(url, confirmConfig);
    }
    if (browserWallet) {
      //@ts-ignore
      this.connection = connection;
      this.browserWallet = browserWallet;
    }
  }

  private async fetchLookupTable() {
    if (!this.provider) throw new Error("No provider set.");
    // TODO: replace with api call to relayer - lookuptable
    this.lookUpTable = await initLookUpTableFromFile(this.provider);
  }
  private async fetchMerkleTree() {
    // Api call to relayer - merkletree
  }
  private async loadPoseidon() {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    this.poseidon = poseidon;
  }
  async latestMerkleTree() {
    await this.fetchMerkleTree();
  }
  // TODO: add loadEddsa

  /**
   * Only use this if you use the WalletAdapter, e.g. in the browser. If you use a local keypair, use getNodeProvider().
   * @param walletContext get from useWallet() hook
   * @param confirmConfig optional, default = 'confirmed'
   * @param connection get from useConnection() hook
   */
  static async getProvider(
    walletContext?: BrowserWallet,
    connection?: Connection,
    confirmConfig?: ConfirmOptions,
  ): Promise<Provider> {
    const provider = new Provider({
      browserWallet: walletContext,
      confirmConfig,
      connection,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree();
    return provider;
  }

  /**
   * Only use this if you have access to a local keypair. If you use WalletAdapter, e.g. in a browser, use getProvider() instead.
   * @param keypair - user's keypair to sign transactions
   * @param confirmConfig optional, default = 'confirmed'
   * @param url full-node rpc endpoint to instantiate a Connection
   */
  static async getNodeProvider(
    keypair?: SolanaKeypair,
    url?: string,
    confirmConfig?: ConfirmOptions,
  ): Promise<Provider> {
    const provider = new Provider({
      nodeWallet: keypair,
      confirmConfig,
      url,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree();
    return provider;
  }
}
