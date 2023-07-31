import { AnchorProvider, BN, Wallet as AnchorWallet } from "@coral-xyz/anchor";
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
  MINT,
  MINIMUM_LAMPORTS,
  ParsedIndexedTransaction,
} from "../index";

const axios = require("axios");
const circomlibjs = require("circomlibjs");

/**
 * use: signMessage, signTransaction, signAllTransactions, sendAndConfirmTransaction, publicKey from the useWallet() hook in solana/wallet-adapter and {connection} from useConnection()
 */
export type Wallet = {
  connection: Connection;
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  signAllTransactions: (transactions: any[]) => Promise<any>;
  sendAndConfirmTransaction: (transaction: any) => Promise<any>;
  publicKey: PublicKey;
  isNodeWallet?: boolean;
};

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
    url,
    minimumLamports = MINIMUM_LAMPORTS,
    relayer,
    verifierProgramLookupTable,
    assetLookupTable,
    versionedTransactionLookupTable,
  }: {
    wallet: Wallet;
    confirmConfig?: ConfirmOptions;
    url: string;
    minimumLamports?: BN;
    relayer?: Relayer;
    verifierProgramLookupTable?: PublicKey[];
    assetLookupTable?: PublicKey[];
    versionedTransactionLookupTable: PublicKey;
  }) {
    if (!wallet)
      throw new ProviderError(
        ProviderErrorCode.WALLET_UNDEFINED,
        "constructor",
        "No wallet provided.",
      );

    const anchorProvider = new AnchorProvider(
      wallet.connection,
      wallet,
      AnchorProvider.defaultOptions(),
    );
    this.provider = anchorProvider;
    this.wallet = wallet;
    this.confirmConfig = confirmConfig || { commitment: "confirmed" };
    this.minimumLamports = minimumLamports;
    this.url = url;
    this.connection = wallet.connection;
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
    relayerUrl?: string,
  ): Promise<PublicKey | undefined> {
    if (wallet.isNodeWallet) {
      return await initLookUpTable(wallet);
    } else {
      if (!relayerUrl)
        throw new ProviderError(
          ProviderErrorCode.URL_UNDEFINED,
          "fetchLookupTable",
        );
      const response = await axios.get(relayerUrl + "/lookuptable");
      return new PublicKey(response.data.data);
    }
  }

  // TODO: extend with: remote-fetching the merkletree from indexer.
  private async fetchMerkleTree(
    merkleTreePubkey: PublicKey,
    indexedTransactions?: ParsedIndexedTransaction[],
  ) {
    try {
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

  async latestMerkleTree(indexedTransactions?: ParsedIndexedTransaction[]) {
    await this.fetchMerkleTree(
      TRANSACTION_MERKLE_TREE_KEY,
      indexedTransactions,
    );
  }

  /**
   * @param wallet get from useWallet() hook and useConnection() hook in browser, or pass in a solana web3 wallet in nodejs context.
   * @param url full-node rpc endpoint to instantiate a Connection, only needed for for nodejs context.
   */
  static async init({
    wallet,
    confirmConfig,
    url = "http://127.0.0.1:8899",
    relayer,
    assetLookupTable,
    verifierProgramLookupTable,
    versionedTransactionLookupTable,
  }: {
    wallet: Wallet | SolanaKeypair | Keypair;
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
    if ("secretKey" in wallet) {
      wallet = useWallet(wallet as SolanaKeypair, url);
    } else {
      wallet = wallet as Wallet;
    }

    if (!versionedTransactionLookupTable) {
      // initializing lookup table or fetching one from relayer in case of browser wallet
      versionedTransactionLookupTable = await Provider.fetchLookupTable(
        wallet,
        relayer?.url,
      );
    } else {
      // checking that lookup table is initialized
      try {
        const lookupTableAccount = await wallet.connection.getAccountInfo(
          versionedTransactionLookupTable,
          "confirmed",
        );
        if (!lookupTableAccount) {
          throw new ProviderError(
            ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED,
            "init",
          );
        }
        // this will throw if the account is not a valid lookup table
        AddressLookupTableAccount.deserialize(lookupTableAccount.data);
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
      url,
      relayer,
      assetLookupTable,
      verifierProgramLookupTable,
      versionedTransactionLookupTable,
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
