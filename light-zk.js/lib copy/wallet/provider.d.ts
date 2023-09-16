/// <reference types="bn.js" />
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { PublicKey, Keypair as SolanaKeypair, Connection, ConfirmOptions, Keypair } from "@solana/web3.js";
import { Relayer, SolMerkleTree, ParsedIndexedTransaction } from "../index";
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
export declare class Provider {
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
    constructor({ wallet, confirmConfig, connection, url, minimumLamports, relayer, verifierProgramLookupTable, assetLookupTable, versionedTransactionLookupTable, anchorProvider, }: {
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
    });
    static loadMock(): Promise<Provider>;
    static fetchLookupTable(wallet: Wallet, provider: AnchorProvider, relayerUrl?: string): Promise<PublicKey | undefined>;
    private fetchMerkleTree;
    private loadPoseidon;
    latestMerkleTree(indexedTransactions?: ParsedIndexedTransaction[]): Promise<void>;
    /**
     * Only use this if you use the WalletAdapter, e.g. in the browser. If you use a local keypair, use getNodeProvider().
     * @param walletContext get from useWallet() hook
     * @param confirmConfig optional, default = 'confirmed'
     * @param connection get from useConnection() hook
     * @param url full-node rpc endpoint to instantiate a Connection
     */
    static init({ wallet, connection, confirmConfig, url, relayer, assetLookupTable, verifierProgramLookupTable, versionedTransactionLookupTable, }: {
        wallet: Wallet | SolanaKeypair | Keypair;
        connection?: Connection;
        confirmConfig: ConfirmOptions;
        url?: string;
        relayer?: Relayer;
        assetLookupTable?: PublicKey[];
        verifierProgramLookupTable?: PublicKey[];
        versionedTransactionLookupTable?: PublicKey;
    }): Promise<Provider>;
    addVerifierProgramPublickeyToLookUpTable(address: PublicKey): void;
    addAssetPublickeyToLookUpTable(address: PublicKey): void;
}
