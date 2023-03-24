import {
  AnchorError,
  AnchorProvider,
  BN,
  setProvider,
} from "@coral-xyz/anchor";
import { SolMerkleTree } from "../merkleTree";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
  Keypair,
} from "@solana/web3.js";
import { ADMIN_AUTH_KEYPAIR, initLookUpTableFromFile } from "../test-utils";
import { MERKLE_TREE_HEIGHT, MERKLE_TREE_KEY } from "../constants";
import { MerkleTree } from "../merkleTree/merkleTree";
import { ProviderError, ProviderErrorCode } from "../errors";
import { useWallet } from "./useWallet";
const axios = require("axios");
const circomlibjs = require("circomlibjs");

/**
 * use: signMessage, signTransaction, sendAndConfirmTransaction, publicKey from the useWallet() hook in solana/wallet-adapter and {connection} from useConnection()
 */
export type Wallet = {
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  sendAndConfirmTransaction: (transaction: any) => Promise<any>;
  publicKey: PublicKey;
  isNodeWallet?: boolean;
};

/**
 * Provides: wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add relayer here; default deriv, if passed in can choose custom relayer.
export class Provider {
  connection?: Connection;
  wallet: Wallet;
  confirmConfig: ConfirmOptions;
  poseidon: any;
  lookUpTable?: PublicKey;
  solMerkleTree?: SolMerkleTree;
  provider?: AnchorProvider | { connection: Connection }; // temp -?
  url?: string;
  minimumLamports: BN;

  /**
   * Initialize with Wallet or SolanaKeypair. Feepayer is the provided wallet
   * Optionally provide confirmConfig, Default = 'confirmed'.
   */
  constructor({
    wallet,
    confirmConfig,
    connection,
    url = "http://127.0.0.1:8899",
    minimumLamports = new BN(5000 * 30),
  }: {
    wallet: Wallet | SolanaKeypair;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url?: string;
    minimumLamports?: BN;
  }) {
    if (!wallet)
      throw new ProviderError(
        ProviderErrorCode.WALLET_UNDEFINED,
        "constructor",
        "No wallet provided.",
      );

    if (!("secretKey" in wallet) && !connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_UNDEFINED,
        "constructor",
        "No connection provided with browser wallet.",
      );
    if ("secretKey" in wallet && connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_DEFINED,
        "constructor",
        "Connection provided in node environment. Provide a url instead",
      );

    this.confirmConfig = confirmConfig || { commitment: "confirmed" };
    this.minimumLamports = minimumLamports;

    if ("secretKey" in wallet) {
      this.wallet = useWallet(wallet as SolanaKeypair, url);
      // TODO: check if we can remove this.url!
      this.url = url;
      // TODO: check if we can remove this.provider!
      if (url !== "mock") {
        setProvider(AnchorProvider.env());
        this.provider = AnchorProvider.local(url, confirmConfig);
      }
    } else {
      this.connection = connection;
      this.provider = { connection: connection! };
      this.wallet = wallet as Wallet;
    }
  }

  static async loadMock() {
    let mockProvider = new Provider({
      wallet: ADMIN_AUTH_KEYPAIR,
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
      if (!this.wallet.isNodeWallet) {
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
      if (!this.wallet.isNodeWallet) {
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
        "confirmed",
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
   * @param url full-node rpc endpoint to instantiate a Connection
   */
  static async init(
    wallet: Wallet | SolanaKeypair | Keypair,
    connection?: Connection,
    confirmConfig?: ConfirmOptions,
    url?: string,
  ): Promise<Provider> {
    if (!wallet) {
      throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
    }
    const provider = new Provider({
      wallet,
      confirmConfig,
      connection,
      url,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree(MERKLE_TREE_KEY);
    return provider;
  }
}
