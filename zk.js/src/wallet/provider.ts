import { AnchorProvider, BN } from "@coral-xyz/anchor";
import {
  AddressLookupTableAccount,
  ConfirmOptions,
  Connection,
  Keypair,
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  AddressLookupTableAccountArgs,
  Commitment,
  BlockhashWithExpiryBlockHeight,
  TransactionSignature,
  VersionedTransaction,
  TransactionConfirmationStrategy,
} from "@solana/web3.js";
import { initLookUpTable } from "../utils";
import {
  ADMIN_AUTH_KEYPAIR,
  MINIMUM_LAMPORTS,
  MINT,
  PrioritizationFee,
  ProviderError,
  ProviderErrorCode,
  Relayer,
  RELAYER_FEE,
  RELAYER_RECIPIENT_KEYPAIR,
  RelayerSendTransactionsResponse,
  sendVersionedTransactions,
  SendVersionedTransactionsResult,
  TestRelayer,
  TOKEN_ACCOUNT_FEE,
  useWallet,
} from "../index";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { createSolanaTransactions } from "../transaction/createSolanaTransactions";
const axios = require("axios");

/**
 * use: signMessage, signTransaction, sendAndConfirmTransaction, publicKey from the useWallet() hook in solana/wallet-adapter and {connection} from useConnection()
 */
export type Wallet = {
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  signAllTransactions: (transaction: any[]) => Promise<any[]>;
  sendTransaction: (transaction: any, connection?: Connection) => Promise<any>;
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
  hasher: Hasher;
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
    hasher,
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
    hasher: Hasher;
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
        RELAYER_FEE,
        TOKEN_ACCOUNT_FEE,
      );
    }
    const tmpAssetLookupTable = assetLookupTable
      ? [...assetLookupTable.map((entry) => entry.toBase58())]
      : [];

    const tmpVerifierProgramLookupTable = verifierProgramLookupTable
      ? [...verifierProgramLookupTable.map((entry) => entry.toBase58())]
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
    this.hasher = hasher;
  }

  static async loadMock(): Promise<Provider> {
    const hasher = await WasmHasher.getInstance();
    // @ts-ignore: @ananas-block ignoring errors to not pass anchorProvider
    const mockProvider = new Provider({
      wallet: useWallet(ADMIN_AUTH_KEYPAIR),
      url: "mock",
      versionedTransactionLookupTable: PublicKey.default,
      hasher,
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

  /**
   *
   * @param commitment The level of commitment desired when querying the account state by (default: 'confirmed')
   * @returns
   */
  async getVersionedTransactionLookupTableAccountArgs(
    commitment?: Commitment,
  ): Promise<AddressLookupTableAccountArgs> {
    if (!this.connection)
      throw new ProviderError(
        ProviderErrorCode.CONNECTION_UNDEFINED,
        "getKeyedAddressLookUpTableAccountInfo",
      );

    const { versionedTransactionLookupTable } = this.lookUpTables;

    const lookupTableAccount = await this.connection.getAccountInfo(
      versionedTransactionLookupTable,
      // TODO: Determine whether we should use 'finalized' instead:
      // https://docs.solana.com/proposals/versioned-transactions#front-running
      // potential security implications
      commitment || "confirmed",
    );

    const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
      lookupTableAccount!.data,
    );

    return {
      key: versionedTransactionLookupTable,
      state: unpackedLookupTableAccount,
    };
  }

  /**
   * Convenience wrapper for sending and confirming light transactions.
   * Fetches recentBlockhash if none provided, builds transactions from instructions, signs, sends, and confirms them.
   */
  /// TODO: figure out whether we should use confirmOptions to specify the latestblockhash commitment or not
  async sendAndConfirmSolanaInstructions(
    ixs: TransactionInstruction[],
    confirmOptions?: ConfirmOptions,
    prioritizationFee?: PrioritizationFee,
    blockhashInfo?: BlockhashWithExpiryBlockHeight,
    /** The level of commitment desired for querying the blockhash and lookuptable args */
    commitment?: Commitment,
  ): Promise<TransactionSignature[]> {
    const connection = this.connection!;
    const wallet = this.wallet;
    const versionedTransactionLookupTableAccountArgs =
      await this.getVersionedTransactionLookupTableAccountArgs(commitment);

    const { blockhash, lastValidBlockHeight }: BlockhashWithExpiryBlockHeight =
      blockhashInfo || (await connection.getLatestBlockhash(commitment));

    const txs = createSolanaTransactions(
      ixs,
      blockhash,
      versionedTransactionLookupTableAccountArgs,
      prioritizationFee,
    );

    /// TODO: wallet adapter should have a signAndSendAllTransactions interface soon
    const signedTransactions: VersionedTransaction[] =
      await wallet.signAllTransactions(txs);

    const signatures: TransactionSignature[] = [];

    try {
      for (const tx of signedTransactions) {
        const signature: TransactionSignature =
          await connection!.sendTransaction(tx);

        /// we assume that we're able to fit all txs into one blockhash expiry window
        const strategy: TransactionConfirmationStrategy = {
          signature,
          lastValidBlockHeight,
          blockhash,
        };

        await connection.confirmTransaction(
          strategy,
          confirmOptions?.commitment || "confirmed",
        );

        signatures.push(signature);
      }
    } catch (error) {
      console.error("sendAndConfirmSolanaInstructions error: ", error); // TODO: turn into custom error that prints the entire call stack
      throw error;
    }

    return signatures;
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

    const anchorProvider = new AnchorProvider(
      connection,
      wallet,
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

    const hasher = await WasmHasher.getInstance();
    return new Provider({
      wallet,
      confirmConfig,
      connection,
      url,
      relayer,
      assetLookupTable,
      verifierProgramLookupTable,
      versionedTransactionLookupTable,
      anchorProvider,
      hasher,
    });
  }

  addVerifierProgramPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.verifierProgramLookupTable.push(address.toBase58());
  }

  addAssetPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.assetLookupTable.push(address.toBase58());
  }
}
