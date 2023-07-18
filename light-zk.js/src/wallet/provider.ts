import {
  AnchorError,
  AnchorProvider,
  BN,
  Wallet as AnchorWallet,
} from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair as SolanaKeypair,
  Connection,
  ConfirmOptions,
  Keypair,
  SystemProgram,
  AddressLookupTableAccount,
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
  SolMerkleTree,
  RELAYER_RECIPIENT_KEYPAIR,
  IndexedTransaction,
  MINT,
  MINIMUM_LAMPORTS,
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
  solMerkleTree?: SolMerkleTree;
  provider: AnchorProvider;
  url?: string;
  minimumLamports: BN;
  relayer: Relayer;
  lookUpTables: {
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
    versionedTransactionLookupTable: PublicKey;
  };

  /**
   * Initialize with Wallet or SolanaKeypair. Feepayer is the provided wallet
   * Optionally provide confirmConfig, Default = 'confirmed'.
   */
  constructor({
    wallet,
    confirmConfig,
    connection,
    url,
    minimumLamports = MINIMUM_LAMPORTS,
    relayer,
    verifierProgramLookupTable,
    assetLookupTable,
    versionedTransactionLookupTable,
    anchorProvider,
  }: {
    wallet: Wallet;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url: string;
    minimumLamports?: BN;
    relayer?: Relayer;
    verifierProgramLookupTable?: PublicKey[];
    assetLookupTable?: PublicKey[];
    versionedTransactionLookupTable: PublicKey;
    anchorProvider: AnchorProvider;
  }) {
    if (!wallet)
      throw new ProviderError(
        ProviderErrorCode.WALLET_UNDEFINED,
        "constructor",
        "No wallet provided.",
      );
    this.provider = anchorProvider;
    this.wallet = wallet;
    this.confirmConfig = confirmConfig || { commitment: "confirmed" };
    this.minimumLamports = minimumLamports;
    this.url = url;
    this.connection = connection;
    if (relayer) {
      this.relayer = relayer;
    } else {
      this.relayer = new Relayer(
        this.wallet.publicKey,
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
      versionedTransactionLookupTable,
    };
  }

  static async loadMock() {
    // @ts-ignore: @ananas-block ignoring errors to not pass anchorProvider
    let mockProvider = new Provider({
      wallet: useWallet(ADMIN_AUTH_KEYPAIR),
      url: "mock",
      versionedTransactionLookupTable: PublicKey.default,
    });

    await mockProvider.loadPoseidon();
    mockProvider.solMerkleTree = new SolMerkleTree({
      poseidon: mockProvider.poseidon,
      pubkey: TRANSACTION_MERKLE_TREE_KEY,
    });

    return mockProvider;
  }

  static async fetchLookupTable(
    wallet: Wallet,
    provider: AnchorProvider,
    relayerUrl?: string,
  ): Promise<PublicKey | undefined> {
    if (wallet.isNodeWallet) {
      return await initLookUpTable(wallet, provider);
    } else if (relayerUrl) {
      const response = await axios.get(relayerUrl + "/lookuptable");
      return new PublicKey(response.data.data);
    }
  }

  private async fetchMerkleTree(
    merkleTreePubkey: PublicKey,
    indexedTransactions?: IndexedTransaction[],
  ) {
    try {
      if (!this.wallet.isNodeWallet) {
        const response = await axios.get(this.relayer.url + "/merkletree");

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

  async latestMerkleTree(indexedTransactions?: IndexedTransaction[]) {
    await this.fetchMerkleTree(
      TRANSACTION_MERKLE_TREE_KEY,
      indexedTransactions,
    );
  }

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
    url = "http://127.0.0.1:8899",
    relayer,
    assetLookupTable,
    verifierProgramLookupTable,
    versionedTransactionLookupTable,
  }: {
    wallet: Wallet | SolanaKeypair | Keypair;
    connection?: Connection;
    confirmConfig: ConfirmOptions;
    url?: string;
    relayer?: Relayer;
    assetLookupTable?: PublicKey[];
    verifierProgramLookupTable?: PublicKey[];
    versionedTransactionLookupTable?: PublicKey;
  }): Promise<Provider> {
    if (!wallet) {
      throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
    }
    if (!connection) {
      connection = new Connection(url, "confirmed");
    }
    if (!("secretKey" in wallet) && !connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_UNDEFINED,
        "constructor",
        "No connection provided with browser wallet.",
      );
    if ("secretKey" in wallet) {
      wallet = useWallet(wallet as SolanaKeypair, url);
    } else {
      wallet = wallet as Wallet;
    }
    // initializing anchor provider with dummy wallet we are never going to use it to send transactions
    const anchorProvider = new AnchorProvider(
      connection,
      new AnchorWallet(SolanaKeypair.generate()),
      confirmConfig,
    );

    if (!versionedTransactionLookupTable) {
      // initializing lookup table or fetching one from relayer in case of browser wallet
      versionedTransactionLookupTable = await Provider.fetchLookupTable(
        wallet,
        anchorProvider,
        relayer?.url,
      );
    } else {
      // checking that lookup table is initialized
      try {
        const lookupTableAccount = await connection.getAccountInfo(
          versionedTransactionLookupTable,
          "confirmed",
        );
        if (!lookupTableAccount) {
          throw new ProviderError(
            ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED,
            "init",
          );
        }
        const unpackedLookupTableAccount =
          AddressLookupTableAccount.deserialize(lookupTableAccount.data);
        console.log(
          "unpackedLookupTableAccount ------------------------------",
          unpackedLookupTableAccount,
        );
      } catch (error) {
        throw new ProviderError(
          ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED,
          "init",
          `${error}`,
        );
      }
    }
    if (!versionedTransactionLookupTable)
      throw new ProviderError(
        ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED,
        "init",
        "Initializing lookup table in node.js or fetching it from relayer in browser failed",
      );

    const provider = new Provider({
      wallet,
      confirmConfig,
      connection,
      url,
      relayer,
      assetLookupTable,
      verifierProgramLookupTable,
      versionedTransactionLookupTable,
      anchorProvider,
    });

    await provider.loadPoseidon();
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
