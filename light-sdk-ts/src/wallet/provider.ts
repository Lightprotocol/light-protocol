import {
  AnchorError,
  AnchorProvider,
  BN,
  setProvider,
} from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
  Keypair,
} from "@solana/web3.js";
import {
  ProviderError,
  ProviderErrorCode,
  MerkleTree,
  useWallet,
  Relayer,
  MERKLE_TREE_HEIGHT,
  TRANSACTION_MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  initLookUpTableFromFile,
  SolMerkleTree,
  RELAYER_RECIPIENT_KEYPAIR,
} from "../index";

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
  provider?: AnchorProvider;
  url?: string;
  minimumLamports: BN;
  relayer: Relayer;

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
    relayer,
  }: {
    wallet: Wallet | SolanaKeypair;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url?: string;
    minimumLamports?: BN;
    relayer?: Relayer;
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
      this.wallet = wallet as Wallet;
    }

    if (relayer) {
      this.relayer = relayer;
    } else {
      this.relayer = new Relayer(
        this.wallet!.publicKey,
        this.lookUpTable!,
        RELAYER_RECIPIENT_KEYPAIR.publicKey,
        new BN(100000),
      );
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
      pubkey: TRANSACTION_MERKLE_TREE_KEY,
    });

    return mockProvider;
  }

  private async fetchLookupTable() {
    try {
      if (!this.wallet.isNodeWallet) {
        const response = await axios.get("http://localhost:3331/lookuptable");
        const lookUpTable = new PublicKey(response.data.data);
        this.lookUpTable = lookUpTable;
        this.relayer.accounts.lookUpTable = lookUpTable;
        return;
      }
      if (!this.provider) throw new Error("No provider set.");
      // TODO: remove this should not exist
      const lookUpTable = await initLookUpTableFromFile(this.provider);
      this.lookUpTable = lookUpTable;
      this.relayer.accounts.lookUpTable = lookUpTable;
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

      const indexedTransactions = await this.relayer.getIndexedTransactions(
        this.provider!.connection,
      );

      const mt = await SolMerkleTree.build({
        pubkey: merkleTreePubkey,
        poseidon: this.poseidon,
        indexedTransactions,
        provider: this.provider,
      });

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
    await this.fetchMerkleTree(TRANSACTION_MERKLE_TREE_KEY);
  }
  // TODO: add loadEddsa

  /**
   * Only use this if you use the WalletAdapter, e.g. in the browser. If you use a local keypair, use getNodeProvider().
   * @param walletContext get from useWallet() hook
   * @param confirmConfig optional, default = 'confirmed'
   * @param connection get from useConnection() hook
   * @param url full-node rpc endpoint to instantiate a Connection
   */
  static async init({
    wallet,
    connection,
    confirmConfig,
    url,
    relayer,
  }: {
    wallet: Wallet | SolanaKeypair | Keypair;
    connection?: Connection;
    confirmConfig?: ConfirmOptions;
    url?: string;
    relayer?: Relayer;
  }): Promise<Provider> {
    if (!wallet) {
      throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
    }
    const provider = new Provider({
      wallet,
      confirmConfig,
      connection,
      url,
      relayer,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree(TRANSACTION_MERKLE_TREE_KEY);
    return provider;
  }
}
