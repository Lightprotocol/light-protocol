import { AnchorError, AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { SolMerkleTree } from "../merkleTree";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
} from "@solana/web3.js";
import {
  initLookUpTable,
  initLookUpTableFromFile,
  setUpMerkleTree,
} from "../test-utils";
import { MERKLE_TREE_KEY } from "../constants";
const axios = require("axios");
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
 * Provides: wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add relayer here; default deriv, if passed in can choose custom relayer.
export class Provider {
  connection?: Connection;
  browserWallet?: BrowserWallet;
  nodeWallet?: SolanaKeypair;
  confirmConfig: ConfirmOptions;
  poseidon: any;
  lookUpTable?: PublicKey;
  solMerkleTree?: SolMerkleTree;
  provider?: AnchorProvider; // temp -?
  url?: string;

  /**
   * Init either with nodeWallet or browserWallet. Feepayer is the provided wallet
   * Optionally provide confirmConfig, Default = 'confirmed'.
   */
  constructor({
    nodeWallet,
    browserWallet,
    confirmConfig,
    connection,
    url = "http://127.0.0.1:8899",
  }: {
    nodeWallet?: SolanaKeypair;
    browserWallet?: BrowserWallet;
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

    this.confirmConfig = confirmConfig || { commitment: "confirmed" };

    if (nodeWallet) {
      this.nodeWallet = nodeWallet;
      // TODO: check if we can remove this.url!
      this.url = url;
      // TODO: check if we can remove this.provider!
      if (url !== "mock") {
        setProvider(AnchorProvider.env());
        this.provider = AnchorProvider.local(url, confirmConfig);
      }
    }
    if (browserWallet) {
      //@ts-ignore
      this.connection = connection;
      this.browserWallet = browserWallet;
    }
  }

  static async loadMock(mockPubkey: PublicKey) {
    let mockProvider = new Provider({
      nodeWallet: SolanaKeypair.generate(),
      url: "mock",
    });
    await mockProvider.loadPoseidon();
    mockProvider.lookUpTable = SolanaKeypair.generate().publicKey;
    mockProvider.solMerkleTree = new SolMerkleTree({
      poseidon: mockProvider.poseidon,
      pubkey: mockPubkey,
    });

    return mockProvider;
  }

  private async fetchLookupTable() {
    if (this.browserWallet) {
      const response = await axios.get("http://localhost:3331/lookuptable");
      this.lookUpTable = response.data;
      console.log(
        "lookuptable fetched from 3331",
        this.lookUpTable,
        response.data,
      );
      return;
    }
    if (!this.provider) throw new Error("No provider set.");
    this.lookUpTable = await initLookUpTableFromFile(this.provider);
  }

  private async fetchMerkleTree() {
    if (this.browserWallet) {
      const response = await axios.get("http://localhost:3331/merkletree");
      this.solMerkleTree = response.data;
      console.log(
        "merkletree fetched from 3331",
        this.solMerkleTree,
        response.data,
      );
      return;
    }
    // TODO: move to a seperate function
    const merkletreeIsInited = await this.provider!.connection.getAccountInfo(
      MERKLE_TREE_KEY,
    );
    if (!merkletreeIsInited) {
      await setUpMerkleTree(this.provider!);
      console.log("merkletree inited");
      // TODO: throw error
    }

    console.log("building merkletree...");
    const mt = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: this.poseidon,
    });
    console.log("✔️ building merkletree done");
    this.solMerkleTree = mt;
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
  static async browser(
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
  static async native(
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
