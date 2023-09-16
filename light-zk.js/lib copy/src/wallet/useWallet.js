"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.useWallet = void 0;
const web3_js_1 = require("@solana/web3.js");
const tweetnacl_1 = require("tweetnacl");
// Mock Solana web3 library
class Wallet {
    constructor(keypair, url, commitment) {
        this.signTransaction = async (tx) => {
            await tx.sign([this._keypair]);
            return tx;
        };
        this.signAllTransactions = async (transactions) => {
            const signedTxs = await Promise.all(transactions.map(async (tx) => {
                return await this.signTransaction(tx);
            }));
            return signedTxs;
        };
        this.signMessage = async (message) => {
            return tweetnacl_1.sign.detached(message, this._keypair.secretKey);
        };
        this.sendAndConfirmTransaction = async (transaction, signers = []) => {
            const response = await (0, web3_js_1.sendAndConfirmTransaction)(this._connection, transaction, [this._keypair, ...signers], {
                commitment: this._commitment,
            });
            return response;
        };
        this._publicKey = keypair.publicKey;
        this._keypair = keypair;
        this._connection = new web3_js_1.Connection(url);
        this._url = url;
        this._commitment = commitment;
    }
}
// Mock useWallet hook
const useWallet = (keypair, url = "http://127.0.0.1:8899", isNodeWallet = true, commitment = "confirmed") => {
    url = url !== "mock" ? url : "http://127.0.0.1:8899";
    const wallet = new Wallet(keypair, url, commitment);
    return {
        publicKey: wallet._publicKey,
        sendAndConfirmTransaction: wallet.sendAndConfirmTransaction,
        signMessage: wallet.signMessage,
        signTransaction: wallet.signTransaction,
        signAllTransactions: wallet.signAllTransactions,
        isNodeWallet,
    };
};
exports.useWallet = useWallet;
//# sourceMappingURL=useWallet.js.map