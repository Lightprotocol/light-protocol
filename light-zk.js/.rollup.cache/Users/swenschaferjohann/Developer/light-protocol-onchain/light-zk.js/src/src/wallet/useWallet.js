import { Connection, sendAndConfirmTransaction, } from "@solana/web3.js";
import { sign } from "tweetnacl";
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
            return sign.detached(message, this._keypair.secretKey);
        };
        this.sendAndConfirmTransaction = async (transaction, signers = []) => {
            const response = await sendAndConfirmTransaction(this._connection, transaction, [this._keypair, ...signers], {
                commitment: this._commitment,
            });
            return response;
        };
        this._publicKey = keypair.publicKey;
        this._keypair = keypair;
        this._connection = new Connection(url);
        this._url = url;
        this._commitment = commitment;
    }
}
// Mock useWallet hook
export const useWallet = (keypair, url = "http://127.0.0.1:8899", isNodeWallet = true, commitment = "confirmed") => {
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
//# sourceMappingURL=useWallet.js.map