import { AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey, Connection, SystemProgram, AddressLookupTableAccount, } from "@solana/web3.js";
import { initLookUpTable } from "../utils";
import { ProviderError, ProviderErrorCode, useWallet, Relayer, ADMIN_AUTH_KEYPAIR, SolMerkleTree, RELAYER_RECIPIENT_KEYPAIR, MINT, MINIMUM_LAMPORTS, MerkleTreeConfig, RELAYER_FEE, } from "../index";
const axios = require("axios");
const circomlibjs = require("circomlibjs");
/**
 * Provides: wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add relayer here; default deriv, if passed in can choose custom relayer.
export class Provider {
    /**
     * Initialize with Wallet or SolanaKeypair. Feepayer is the provided wallet
     * Optionally provide confirmConfig, Default = 'confirmed'.
     */
    constructor({ wallet, confirmConfig, connection, url, minimumLamports = MINIMUM_LAMPORTS, relayer, verifierProgramLookupTable, assetLookupTable, versionedTransactionLookupTable, anchorProvider, }) {
        if (!wallet)
            throw new ProviderError(ProviderErrorCode.WALLET_UNDEFINED, "constructor", "No wallet provided.");
        this.provider = anchorProvider;
        this.wallet = wallet;
        this.confirmConfig = confirmConfig || { commitment: "confirmed" };
        this.minimumLamports = minimumLamports;
        this.url = url;
        this.connection = connection;
        if (relayer) {
            this.relayer = relayer;
        }
        else {
            this.relayer = new Relayer(this.wallet.publicKey, RELAYER_RECIPIENT_KEYPAIR.publicKey, RELAYER_FEE);
        }
        let tmpAssetLookupTable = assetLookupTable
            ? [...assetLookupTable === null || assetLookupTable === void 0 ? void 0 : assetLookupTable.map((entry) => entry.toBase58())]
            : [];
        let tmpVerifierProgramLookupTable = verifierProgramLookupTable
            ? [...verifierProgramLookupTable === null || verifierProgramLookupTable === void 0 ? void 0 : verifierProgramLookupTable.map((entry) => entry.toBase58())]
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
            pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
        });
        return mockProvider;
    }
    static async fetchLookupTable(wallet, provider, relayerUrl) {
        if (wallet.isNodeWallet) {
            return await initLookUpTable(wallet, provider);
        }
        else if (relayerUrl) {
            const response = await axios.get(relayerUrl + "/lookuptable");
            return new PublicKey(response.data.data);
        }
    }
    async fetchMerkleTree(merkleTreePubkey, indexedTransactions) {
        try {
            const merkletreeIsInited = await this.provider.connection.getAccountInfo(merkleTreePubkey, "confirmed");
            if (!merkletreeIsInited) {
                throw new ProviderError(ProviderErrorCode.MERKLE_TREE_NOT_INITIALIZED, "fetchMerkleTree", `Merkle tree is not initialized if on local host run test utils setUpMerkleTree before initin the provider, on other networks check your merkle tree pubkey ${merkleTreePubkey}`);
            }
            if (!indexedTransactions) {
                indexedTransactions = await this.relayer.getIndexedTransactions(this.provider.connection);
            }
            const mt = await SolMerkleTree.build({
                pubkey: merkleTreePubkey,
                poseidon: this.poseidon,
                indexedTransactions,
                provider: this.provider,
            });
            this.solMerkleTree = mt;
        }
        catch (err) {
            console.error(err);
            throw err;
        }
    }
    async loadPoseidon() {
        const poseidon = await circomlibjs.buildPoseidonOpt();
        this.poseidon = poseidon;
    }
    async latestMerkleTree(indexedTransactions) {
        await this.fetchMerkleTree(MerkleTreeConfig.getTransactionMerkleTreePda(), indexedTransactions);
    }
    /**
     * Only use this if you use the WalletAdapter, e.g. in the browser. If you use a local keypair, use getNodeProvider().
     * @param walletContext get from useWallet() hook
     * @param confirmConfig optional, default = 'confirmed'
     * @param connection get from useConnection() hook
     * @param url full-node rpc endpoint to instantiate a Connection
     */
    static async init({ wallet, connection, confirmConfig, url = "http://127.0.0.1:8899", relayer, assetLookupTable, verifierProgramLookupTable, versionedTransactionLookupTable, }) {
        if (!wallet) {
            throw new ProviderError(ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
        }
        if (!connection) {
            connection = new Connection(url, "confirmed");
        }
        if (!("secretKey" in wallet) && !connection)
            throw new ProviderError(ProviderErrorCode.CONNECTION_UNDEFINED, "constructor", "No connection provided with browser wallet.");
        if ("secretKey" in wallet) {
            wallet = useWallet(wallet, url);
        }
        else {
            wallet = wallet;
        }
        const anchorProvider = new AnchorProvider(connection, wallet, confirmConfig);
        if (!versionedTransactionLookupTable) {
            // initializing lookup table or fetching one from relayer in case of browser wallet
            versionedTransactionLookupTable = await Provider.fetchLookupTable(wallet, anchorProvider, relayer === null || relayer === void 0 ? void 0 : relayer.url);
        }
        else {
            // checking that lookup table is initialized
            try {
                const lookupTableAccount = await connection.getAccountInfo(versionedTransactionLookupTable, "confirmed");
                if (!lookupTableAccount) {
                    throw new ProviderError(ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init");
                }
                // this will throw if the account is not a valid lookup table
                AddressLookupTableAccount.deserialize(lookupTableAccount.data);
            }
            catch (error) {
                throw new ProviderError(ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init", `${error}`);
            }
        }
        if (!versionedTransactionLookupTable)
            throw new ProviderError(ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init", "Initializing lookup table in node.js or fetching it from relayer in browser failed");
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
        await provider.fetchMerkleTree(MerkleTreeConfig.getTransactionMerkleTreePda());
        return provider;
    }
    addVerifierProgramPublickeyToLookUpTable(address) {
        this.lookUpTables.verifierProgramLookupTable.push(address.toBase58());
    }
    addAssetPublickeyToLookUpTable(address) {
        this.lookUpTables.assetLookupTable.push(address.toBase58());
    }
}
//# sourceMappingURL=provider.js.map