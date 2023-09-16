"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.TestRelayer = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const relayer_1 = require("../relayer");
const updateMerkleTree_1 = require("./updateMerkleTree");
const wallet_1 = require("../wallet");
const transaction_1 = require("../transaction");
const airdrop_1 = require("./airdrop");
const index_1 = require("../index");
class TestRelayer extends relayer_1.Relayer {
    constructor({ relayerPubkey, relayerRecipientSol, relayerFee = index_1.BN_0, highRelayerFee, payer, }) {
        super(relayerPubkey, relayerRecipientSol, relayerFee, highRelayerFee);
        this.indexedTransactions = [];
        if (payer.publicKey.toBase58() != relayerPubkey.toBase58())
            throw new Error(`Payer public key ${payer.publicKey.toBase58()} does not match relayer public key ${relayerPubkey.toBase58()}`);
        this.relayerKeypair = payer;
    }
    async updateMerkleTree(provider) {
        if (!provider.provider)
            throw new Error("Provider.provider is undefined.");
        if (!provider.url)
            throw new Error("Provider.provider is undefined.");
        if (provider.url !== "http://127.0.0.1:8899")
            throw new Error("Provider url is not http://127.0.0.1:8899");
        const balance = await provider.provider.connection?.getBalance(this.relayerKeypair.publicKey);
        if (!balance || balance < 1e9) {
            await (0, airdrop_1.airdropSol)({
                connection: provider.provider.connection,
                lamports: 1000000000,
                recipientPublicKey: this.relayerKeypair.publicKey,
            });
        }
        try {
            const response = await (0, updateMerkleTree_1.updateMerkleTreeForTest)(this.relayerKeypair, provider.url);
            return response;
        }
        catch (e) {
            console.log(e);
            throw e;
        }
    }
    async sendTransactions(instructions, provider) {
        var res = await (0, transaction_1.sendVersionedTransactions)(instructions, provider.provider.connection, provider.lookUpTables.versionedTransactionLookupTable, (0, wallet_1.useWallet)(this.relayerKeypair));
        if (res.error)
            return { transactionStatus: "error", ...res };
        else
            return { transactionStatus: "confirmed", ...res };
    }
    /**
     * Indexes light transactions by:
     * - getting all signatures the merkle tree was involved in
     * - trying to extract and parse event cpi for every signature's transaction
     * - if there are indexed transactions already in the relayer object only transactions after the last indexed event are indexed
     * @param connection
     * @returns
     */
    async getIndexedTransactions(connection) {
        const merkleTreeAccountInfo = await connection.getAccountInfo(index_1.MerkleTreeConfig.getTransactionMerkleTreePda(), "confirmed");
        if (!merkleTreeAccountInfo)
            throw new Error("Failed to fetch merkle tree account");
        const coder = new anchor_1.BorshAccountsCoder(index_1.IDL_MERKLE_TREE_PROGRAM);
        const merkleTreeAccount = coder.decode("transactionMerkleTree", merkleTreeAccountInfo.data);
        // limits the number of signatures which are queried
        // if the number is too low it is not going to index all transactions
        // hence the dependency on the merkle tree account index times 260 transactions
        // which is approximately the number of transactions sent to send one shielded transaction and update the merkle tree
        const limit = 1000 + 260 * merkleTreeAccount.nextIndex.toNumber();
        if (this.indexedTransactions.length === 0) {
            let newTransactions = await (0, transaction_1.fetchRecentTransactions)({
                connection,
                batchOptions: {
                    limit,
                },
            });
            this.indexedTransactions = newTransactions.map((trx) => {
                return {
                    ...trx,
                    firstLeafIndex: new anchor_1.BN(trx.firstLeafIndex, "hex"),
                    publicAmountSol: new anchor_1.BN(trx.publicAmountSol, "hex"),
                    publicAmountSpl: new anchor_1.BN(trx.publicAmountSpl, "hex"),
                    changeSolAmount: new anchor_1.BN(trx.changeSolAmount, "hex"),
                    relayerFee: new anchor_1.BN(trx.relayerFee, "hex"),
                };
            });
            return this.indexedTransactions;
        }
        else {
            if (this.indexedTransactions.length === 0)
                return [];
            let mostRecentTransaction = this.indexedTransactions.reduce((a, b) => a.blockTime > b.blockTime ? a : b);
            let newTransactions = await (0, transaction_1.fetchRecentTransactions)({
                connection,
                batchOptions: {
                    limit,
                    until: mostRecentTransaction.signature,
                },
            });
            let parsedNewTransactions = newTransactions.map((trx) => {
                return {
                    ...trx,
                    firstLeafIndex: new anchor_1.BN(trx.firstLeafIndex, "hex"),
                    publicAmountSol: new anchor_1.BN(trx.publicAmountSol, "hex"),
                    publicAmountSpl: new anchor_1.BN(trx.publicAmountSpl, "hex"),
                    changeSolAmount: new anchor_1.BN(trx.changeSolAmount, "hex"),
                    relayerFee: new anchor_1.BN(trx.relayerFee, "hex"),
                };
            });
            this.indexedTransactions = [
                ...this.indexedTransactions,
                ...parsedNewTransactions,
            ];
            return this.indexedTransactions;
        }
    }
}
exports.TestRelayer = TestRelayer;
//# sourceMappingURL=testRelayer.js.map