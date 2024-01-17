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
} from "@solana/web3.js";
import { initLookUpTable } from "./utils/utils";
import {
  ADMIN_AUTH_KEYPAIR,
  MINIMUM_LAMPORTS,
  MINT,
  ProviderError,
  ProviderErrorCode,
  Rpc,
  RPC_FEE,
  RPC_RECIPIENT_KEYPAIR,
  RpcSendTransactionsResponse,
  sendVersionedTransactions,
  SendVersionedTransactionsResult,
  TOKEN_ACCOUNT_FEE,
  useWallet,
} from "./index";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
const axios = require("axios");

/**
 * use: signMessage, signTransaction, sendAndConfirmTransaction, publicKey from the useWallet() hook in solana/wallet-adapter and {connection} from useConnection()
 */
export type Wallet = {
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  signAllTransactions: (transaction: any[]) => Promise<any[]>;
  sendAndConfirmTransaction: (transaction: any) => Promise<any>;
  publicKey: PublicKey;
  isNodeWallet?: boolean;
};

/**
 * Provides: wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add rpc here; default deriv, if passed in can choose custom rpc.
export class Provider {
  lightWasm: LightWasm;
  connection?: Connection;
  wallet: Wallet;
  confirmConfig: ConfirmOptions;
  provider: AnchorProvider;
  url?: string;
  minimumLamports: BN;
  rpc: Rpc;
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
    lightWasm,
    wallet,
    confirmConfig,
    connection,
    url,
    minimumLamports = MINIMUM_LAMPORTS,
    rpc,
    verifierProgramLookupTable,
    assetLookupTable,
    versionedTransactionLookupTable,
    anchorProvider,
  }: {
    lightWasm: LightWasm;
    wallet: Wallet;
    confirmConfig?: ConfirmOptions;
    connection?: Connection;
    url: string;
    minimumLamports?: BN;
    rpc?: Rpc;
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
    if (rpc) {
      this.rpc = rpc;
    } else {
      this.rpc = new Rpc(
        this.wallet.publicKey,
        RPC_RECIPIENT_KEYPAIR.publicKey,
        RPC_FEE,
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
    this.lightWasm = lightWasm;
  }

  static async loadMock(): Promise<Provider> {
    const lightWasm = await WasmFactory.getInstance();
    // @ts-ignore: @ananas-block ignoring errors to not pass anchorProvider
    const mockProvider = new Provider({
      lightWasm,
      wallet: useWallet(ADMIN_AUTH_KEYPAIR),
      url: "mock",
      versionedTransactionLookupTable: PublicKey.default,
    });

    return mockProvider;
  }

  static async fetchLookupTable(
    wallet: Wallet,
    provider: AnchorProvider,
    rpcUrl?: string,
  ): Promise<PublicKey | undefined> {
    if (wallet.isNodeWallet) {
      return await initLookUpTable(wallet, provider);
    } else if (rpcUrl) {
      const response = await axios.get(rpcUrl + "/lookuptable");
      return new PublicKey(response.data.data);
    }
  }

  async sendAndConfirmTransaction(
    instructions: TransactionInstruction[],
  ): Promise<RpcSendTransactionsResponse | SendVersionedTransactionsResult> {
    const response = await sendVersionedTransactions(
      instructions,
      this.provider.connection,
      this.lookUpTables.versionedTransactionLookupTable,
      this.wallet,
    );
    if (response.error) throw response.error;
    return response;
  }

  async sendAndConfirmCompressedTransaction(
    instructions: TransactionInstruction[],
  ): Promise<RpcSendTransactionsResponse | SendVersionedTransactionsResult> {
    const response = await this.rpc.sendTransactions(instructions, this);
    if (response.error) throw response.error;
    return response;
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
    rpc,
    assetLookupTable,
    verifierProgramLookupTable,
    versionedTransactionLookupTable,
  }: {
    wallet: Wallet | SolanaKeypair | Keypair;
    connection?: Connection;
    confirmConfig: ConfirmOptions;
    url?: string;
    rpc?: Rpc;
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
      // initializing lookup table or fetching one from rpc in case of browser wallet
      versionedTransactionLookupTable = await Provider.fetchLookupTable(
        wallet,
        anchorProvider,
        rpc?.url,
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
        "Initializing lookup table in node.js or fetching it from rpc in browser failed",
      );

    const lightWasm = await WasmFactory.getInstance();
    return new Provider({
      lightWasm,
      wallet,
      confirmConfig,
      connection,
      url,
      rpc,
      assetLookupTable,
      verifierProgramLookupTable,
      versionedTransactionLookupTable,
      anchorProvider,
    });
  }

  addVerifierProgramPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.verifierProgramLookupTable.push(address.toBase58());
  }

  addAssetPublickeyToLookUpTable(address: PublicKey) {
    this.lookUpTables.assetLookupTable.push(address.toBase58());
  }
}
