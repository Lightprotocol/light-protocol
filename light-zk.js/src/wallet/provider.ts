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
  SystemProgram,
} from "@solana/web3.js";
import { initLookUpTable } from "../utils";
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
  IndexedTransaction,
  MINT,
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
  lookUpTables: {
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  };

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
    verifierProgramLookupTable,
    assetLookupTable,
  }: {
    wallet: Wallet | SolanaKeypair;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url?: string;
    minimumLamports?: BN;
    relayer?: Relayer;
    verifierProgramLookupTable?: PublicKey[];
    assetLookupTable?: PublicKey[];
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
    let tmpAssetLookupTable = assetLookupTable
      ? [...assetLookupTable?.map((entry) => entry.toBase58())]
      : [];

    let tmpVerifierProgramLookupTable = verifierProgramLookupTable
      ? [...verifierProgramLookupTable?.map((entry) => entry.toBase58())]
      : [];
    this.lookUpTables = {
      assetLookupTable: [
        SystemProgram.programId.toBase58(),
        MINT.toBase58(),
        ...tmpAssetLookupTable,
      ],
      verifierProgramLookupTable: [
        SystemProgram.programId.toBase58(),
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
        ...tmpVerifierProgramLookupTable,
      ],
    };
  }

  /**
   * Static method to load a mock Provider.
   * - This method is used for testing purposes to generate a mock Provider.
   * - It initializes a Provider with a mock wallet and URL.
   * - It also loads Poseidon hash function and initializes a Solana lookup table and a Solana Merkle Tree.
   *
   * @returns A promise that resolves to a mock Provider.
   */
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
      const lookUpTable = await initLookUpTable(this.wallet, this.provider);
      this.lookUpTable = lookUpTable;
      this.relayer.accounts.lookUpTable = lookUpTable;
    } catch (err) {
      console.error(err);
      throw err;
    }
  }

  private async fetchMerkleTree(
    merkleTreePubkey: PublicKey,
    indexedTransactions?: IndexedTransaction[],
  ) {
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
      if (!indexedTransactions) {
        indexedTransactions = await this.relayer.getIndexedTransactions(
          this.provider!.connection,
        );
      }

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

  /**
   * Fetches the latest Merkle tree by calling the fetchMerkleTree method with the TRANSACTION_MERKLE_TREE_KEY as the key.
   *
   * @param indexedTransactions - An optional array of IndexedTransaction objects. If provided, the fetch operation will use these transactions.
   *
   * @returns {Promise<void>} A Promise that resolves when the operation is completed.
   */
  async latestMerkleTree(indexedTransactions?: IndexedTransaction[]) {
    await this.fetchMerkleTree(
      TRANSACTION_MERKLE_TREE_KEY,
      indexedTransactions,
    );
  }
  // TODO: add loadEddsa

  /**
   * Initializes a Provider instance. This method should only be used if you use the WalletAdapter, such as in the browser.
   * If you use a local keypair, use getNodeProvider() instead.
   *
   * @param wallet - An instance of Wallet or SolanaKeypair or Keypair.
   * @param connection - An optional Connection instance, to be used if already available.
   * @param confirmConfig - An optional ConfirmOptions instance, to configure the confirmation process. By default, it is 'confirmed'.
   * @param url - An optional string, specifying the full-node RPC endpoint to instantiate a Connection.
   * @param relayer - An optional Relayer instance.
   *
   * @throws {ProviderError} Throws an error if the wallet parameter is not provided.
   *
   * @returns {Promise<Provider>} A promise that resolves to a Provider instance.
   *
   * @remarks
   * This method is used to initialize a new instance of the Provider class.
   * - It loads the Poseidon hash function.
   * - Fetches the lookup table.
   * - Fetches the Merkle tree with the TRANSACTION_MERKLE_TREE_KEY.
   * - Finally returns the initialized Provider instance.
   */
  static async init({
    wallet,
    connection,
    confirmConfig,
    url,
    relayer,
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    wallet: Wallet | SolanaKeypair | Keypair;
    connection?: Connection;
    confirmConfig?: ConfirmOptions;
    url?: string;
    relayer?: Relayer;
    assetLookupTable?: PublicKey[];
    verifierProgramLookupTable?: PublicKey[];
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
      assetLookupTable,
      verifierProgramLookupTable,
    });
    await provider.loadPoseidon();
    await provider.fetchLookupTable();
    await provider.fetchMerkleTree(TRANSACTION_MERKLE_TREE_KEY);
    return provider;
  }

  addVerifierProgramPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.verifierProgramLookupTable.push(address.toBase58());
  }

  addAssetPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.assetLookupTable.push(address.toBase58());
  }
}
