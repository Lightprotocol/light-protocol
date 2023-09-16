"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Provider = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
const utils_1 = require("../utils");
const index_1 = require("../index");
const axios = require("axios");
const circomlibjs = require("circomlibjs");
/**
 * Provides: wallets, connection, latest SolMerkleTree, LookupTable, confirmConfig, poseidon
 */
// TODO: add relayer here; default deriv, if passed in can choose custom relayer.
class Provider {
    /**
     * Initialize with Wallet or SolanaKeypair. Feepayer is the provided wallet
     * Optionally provide confirmConfig, Default = 'confirmed'.
     */
    constructor({ wallet, confirmConfig, connection, url, minimumLamports = index_1.MINIMUM_LAMPORTS, relayer, verifierProgramLookupTable, assetLookupTable, versionedTransactionLookupTable, anchorProvider, }) {
        if (!wallet)
            throw new index_1.ProviderError(index_1.ProviderErrorCode.WALLET_UNDEFINED, "constructor", "No wallet provided.");
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
            this.relayer = new index_1.Relayer(this.wallet.publicKey, index_1.RELAYER_RECIPIENT_KEYPAIR.publicKey, index_1.RELAYER_FEE);
        }
        let tmpAssetLookupTable = assetLookupTable
            ? [...assetLookupTable === null || assetLookupTable === void 0 ? void 0 : assetLookupTable.map((entry) => entry.toBase58())]
            : [];
        let tmpVerifierProgramLookupTable = verifierProgramLookupTable
            ? [...verifierProgramLookupTable === null || verifierProgramLookupTable === void 0 ? void 0 : verifierProgramLookupTable.map((entry) => entry.toBase58())]
            : [];
        this.lookUpTables = {
            assetLookupTable: [
                web3_js_1.SystemProgram.programId.toBase58(),
                index_1.MINT.toBase58(),
                ...tmpAssetLookupTable,
            ],
            verifierProgramLookupTable: [
                web3_js_1.SystemProgram.programId.toBase58(),
                "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
                ...tmpVerifierProgramLookupTable,
            ],
            versionedTransactionLookupTable,
        };
    }
    static async loadMock() {
        // @ts-ignore: @ananas-block ignoring errors to not pass anchorProvider
        let mockProvider = new Provider({
            wallet: (0, index_1.useWallet)(index_1.ADMIN_AUTH_KEYPAIR),
            url: "mock",
            versionedTransactionLookupTable: web3_js_1.PublicKey.default,
        });
        await mockProvider.loadPoseidon();
        mockProvider.solMerkleTree = new index_1.SolMerkleTree({
            poseidon: mockProvider.poseidon,
            pubkey: index_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
        });
        return mockProvider;
    }
    static async fetchLookupTable(wallet, provider, relayerUrl) {
        if (wallet.isNodeWallet) {
            return await (0, utils_1.initLookUpTable)(wallet, provider);
        }
        else if (relayerUrl) {
            const response = await axios.get(relayerUrl + "/lookuptable");
            return new web3_js_1.PublicKey(response.data.data);
        }
    }
    async fetchMerkleTree(merkleTreePubkey, indexedTransactions) {
        try {
            const merkletreeIsInited = await this.provider.connection.getAccountInfo(merkleTreePubkey, "confirmed");
            if (!merkletreeIsInited) {
                throw new index_1.ProviderError(index_1.ProviderErrorCode.MERKLE_TREE_NOT_INITIALIZED, "fetchMerkleTree", `Merkle tree is not initialized if on local host run test utils setUpMerkleTree before initin the provider, on other networks check your merkle tree pubkey ${merkleTreePubkey}`);
            }
            if (!indexedTransactions) {
                indexedTransactions = await this.relayer.getIndexedTransactions(this.provider.connection);
            }
            const mt = await index_1.SolMerkleTree.build({
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
        await this.fetchMerkleTree(index_1.MerkleTreeConfig.getTransactionMerkleTreePda(), indexedTransactions);
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
            throw new index_1.ProviderError(index_1.ProviderErrorCode.KEYPAIR_UNDEFINED, "browser");
        }
        if (!connection) {
            connection = new web3_js_1.Connection(url, "confirmed");
        }
        if (!("secretKey" in wallet) && !connection)
            throw new index_1.ProviderError(index_1.ProviderErrorCode.CONNECTION_UNDEFINED, "constructor", "No connection provided with browser wallet.");
        if ("secretKey" in wallet) {
            wallet = (0, index_1.useWallet)(wallet, url);
        }
        else {
            wallet = wallet;
        }
        const anchorProvider = new anchor_1.AnchorProvider(connection, wallet, confirmConfig);
        if (!versionedTransactionLookupTable) {
            // initializing lookup table or fetching one from relayer in case of browser wallet
            versionedTransactionLookupTable = await Provider.fetchLookupTable(wallet, anchorProvider, relayer === null || relayer === void 0 ? void 0 : relayer.url);
        }
        else {
            // checking that lookup table is initialized
            try {
                const lookupTableAccount = await connection.getAccountInfo(versionedTransactionLookupTable, "confirmed");
                if (!lookupTableAccount) {
                    throw new index_1.ProviderError(index_1.ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init");
                }
                // this will throw if the account is not a valid lookup table
                web3_js_1.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
            }
            catch (error) {
                throw new index_1.ProviderError(index_1.ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init", `${error}`);
            }
        }
        if (!versionedTransactionLookupTable)
            throw new index_1.ProviderError(index_1.ProviderErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED, "init", "Initializing lookup table in node.js or fetching it from relayer in browser failed");
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
        await provider.fetchMerkleTree(index_1.MerkleTreeConfig.getTransactionMerkleTreePda());
        return provider;
    }
    addVerifierProgramPublickeyToLookUpTable(address) {
        this.lookUpTables.verifierProgramLookupTable.push(address.toBase58());
    }
    addAssetPublickeyToLookUpTable(address) {
        this.lookUpTables.assetLookupTable.push(address.toBase58());
    }
}
exports.Provider = Provider;
//# sourceMappingURL=provider.js.map