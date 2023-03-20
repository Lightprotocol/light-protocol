import { AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { SolMerkleTree } from "../merkleTree";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
} from "@solana/web3.js";
import { ADMIN_AUTH_KEYPAIR, initLookUpTableFromFile } from "../test-utils";
import { MERKLE_TREE_HEIGHT, MERKLE_TREE_KEY } from "../constants";
import { MerkleTree } from "../merkleTree/merkleTree";
import { ProviderError, ProviderErrorCode } from "../errors";
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
  provider?: AnchorProvider | { connection: Connection }; // temp -?
  url?: string;
  // TODO: refactor by streamlining everything towards using connection and not anchor provider
  // - get rid of url
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
      throw new ProviderError(
        ProviderErrorCode.NODE_WALLET_AND_BROWSER_WALLET_DEFINED,
        "constructor",
        "Both node and browser environments provided chose one.",
      );
    if (!nodeWallet && !browserWallet)
      throw new ProviderError(
        ProviderErrorCode.NODE_WALLET_AND_BROWSER_WALLET_UNDEFINED,
        "constructor",
        "No wallet provided.",
      );
    if (browserWallet && !connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_UNDEFINED,
        "constructor",
        "No connection provided with browser wallet.",
      );
    if (nodeWallet && connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_DEFINED,
        "constructor",
        "Connection provided in node environment. Provide a url instead",
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
      this.provider = { connection: connection! };
      this.browserWallet = browserWallet;
    }
  }

  static async loadMock() {
    let mockProvider = new Provider({
      nodeWallet: ADMIN_AUTH_KEYPAIR,
      url: "mock",
    });
    await mockProvider.loadPoseidon();
    mockProvider.lookUpTable = SolanaKeypair.generate().publicKey;
    mockProvider.solMerkleTree = new SolMerkleTree({
      poseidon: mockProvider.poseidon,
      pubkey: MERKLE_TREE_KEY,
    });

    return mockProvider;
  }

  private async fetchLookupTable() {
    try {
      if (this.browserWallet) {
        const response = await axios.get("http://localhost:3331/lookuptable");
        this.lookUpTable = new PublicKey(response.data.data);
        return;
      }
      if (!this.provider) throw new Error("No provider set.");
      // TODO: remove this should not exist
      this.lookUpTable = await initLookUpTableFromFile(this.provider);
    } catch (err) {
      console.error(err);
      throw err;
    }
  }

  private async fetchMerkleTree(merkleTreePubkey: PublicKey) {
    try {
      if (this.browserWallet) {
        const response = await axios.get("http://localhost:3331/merkletree");

        const fetchedMerkleTree: MerkleTree = response.data.data.merkleTree;

        const pubkey = new PublicKey(response.data.data.pubkey);

        const merkleTree = new MerkleTree(
          MERKLE_TREE_HEIGHT,
          this.poseidon,
          fetchedMerkleTree._layers[0],
        );

        this.solMerkleTree = { ...response.data.data, merkleTree, pubkey };
      }

      const merkletreeIsInited = await this.provider!.connection.getAccountInfo(
        merkleTreePubkey,
      );
      if (!merkletreeIsInited) {
        throw new ProviderError(
          ProviderErrorCode.MERKLE_TREE_NOT_INITIALIZED,
          "fetchMerkleTree",
          `Merkle tree is not initialized if on local host run test utils setUpMerkleTree before initin the provider, on other networks check your merkle tree pubkey ${merkleTreePubkey}`,
        );
      }

      const mt = await SolMerkleTree.build({
        pubkey: merkleTreePubkey,
        poseidon: this.poseidon,
        provider: this.provider,
      });

      console.log("✔️ building merkletree done");
      this.solMerkleTree = mt;
    } catch (err) {
      console.error(err);
      throw err;
    }
  }

  private async loadPoseidon() {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    this.poseidon = poseidon;
  }
  async latestMerkleTree() {
    await this.fetchMerkleTree(MERKLE_TREE_KEY);
  }
  // TODO: add loadEddsa

  /**
   * Only use this if you use the WalletAdapter, e.g. in the browser. If you use a local keypair, use getNodeProvider().
   * @param walletContext get from useWallet() hook
   * @param confirmConfig optional, default = 'confirmed'
   * @param connection get from useConnection() hook
   */
  static async browser(
    browserWallet: BrowserWallet,
    connection: Connection,
    confirmConfig?: ConfirmOptions,
  ): Promise<Provider> {
    if (!browserWallet) {
      throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
    }
    if (!connection) {
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_UNDEFINED,
        "browser",
      );
    }

    const provider = new Provider({
      browserWallet,
      confirmConfig,
      connection,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree(MERKLE_TREE_KEY);
    return provider;
  }

  /**
   * Only use this if you have access to a local keypair. If you use WalletAdapter, e.g. in a browser, use getProvider() instead.
   * @param keypair - user's keypair to sign transactions
   * @param confirmConfig optional, default = 'confirmed'
   * @param url full-node rpc endpoint to instantiate a Connection
   */
  static async native(
    keypair: SolanaKeypair,
    url?: string,
    confirmConfig?: ConfirmOptions,
  ): Promise<Provider> {
    if (!keypair) {
      throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "native");
    }

    const provider = new Provider({
      nodeWallet: keypair,
      confirmConfig,
      url,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree(MERKLE_TREE_KEY);
    return provider;
  }
}
