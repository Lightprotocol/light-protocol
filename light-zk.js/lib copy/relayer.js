"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Relayer = void 0;
const tslib_1 = require("tslib");
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const axios_1 = tslib_1.__importDefault(require("axios"));
const index_1 = require("./index");
class Relayer {
    /**
     *
     * @param relayerPubkey Signs the transaction
     * @param relayerRecipientSol Recipient account for SOL fees
     * @param relayerFee Fee amount
     */
    constructor(relayerPubkey, relayerRecipientSol, relayerFee = index_1.BN_0, highRelayerFee = new anchor_1.BN(index_1.TOKEN_ACCOUNT_FEE), url = "http://localhost:3331") {
        this.indexedTransactions = [];
        if (!relayerPubkey) {
            throw new index_1.RelayerError(index_1.RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED, "constructor");
        }
        if (relayerRecipientSol && relayerFee.eq(index_1.BN_0)) {
            throw new index_1.RelayerError(index_1.RelayerErrorCode.RELAYER_FEE_UNDEFINED, "constructor", "If relayerRecipientSol is defined, relayerFee must be defined and non zero.");
        }
        if (relayerFee.toString() !== "0" && !relayerRecipientSol) {
            throw new index_1.RelayerError(index_1.RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED, "constructor");
        }
        if (relayerRecipientSol) {
            this.accounts = {
                relayerPubkey,
                relayerRecipientSol,
            };
        }
        else {
            this.accounts = {
                relayerPubkey,
                relayerRecipientSol: relayerPubkey,
            };
        }
        this.highRelayerFee = highRelayerFee;
        this.relayerFee = relayerFee;
        this.url = url;
    }
    async updateMerkleTree(_provider) {
        try {
            const response = await axios_1.default.post(this.url + "/updatemerkletree");
            return response;
        }
        catch (err) {
            console.error({ err });
            throw err;
        }
    }
    async sendTransactions(instructions, _provider) {
        try {
            const response = await axios_1.default.post(this.url + "/relayTransaction", {
                instructions,
            });
            return response.data.data;
        }
        catch (err) {
            console.error({ err });
            throw err;
        }
    }
    getRelayerFee(ataCreationFee) {
        return ataCreationFee ? this.highRelayerFee : this.relayerFee;
    }
    async getIndexedTransactions(
    /* We must keep the param for type equality with TestRelayer */
    _connection) {
        try {
            const response = await axios_1.default.get(this.url + "/indexedTransactions");
            const indexedTransactions = response.data.data.map((trx) => {
                return {
                    ...trx,
                    signer: new web3_js_1.PublicKey(trx.signer),
                    to: new web3_js_1.PublicKey(trx.to),
                    from: new web3_js_1.PublicKey(trx.from),
                    toSpl: new web3_js_1.PublicKey(trx.toSpl),
                    fromSpl: new web3_js_1.PublicKey(trx.fromSpl),
                    verifier: new web3_js_1.PublicKey(trx.verifier),
                    relayerRecipientSol: new web3_js_1.PublicKey(trx.relayerRecipientSol),
                    firstLeafIndex: new anchor_1.BN(trx.firstLeafIndex, "hex"),
                    publicAmountSol: new anchor_1.BN(trx.publicAmountSol, "hex"),
                    publicAmountSpl: new anchor_1.BN(trx.publicAmountSpl, "hex"),
                    changeSolAmount: new anchor_1.BN(trx.changeSolAmount, "hex"),
                    relayerFee: new anchor_1.BN(trx.relayerFee, "hex"),
                };
            });
            return indexedTransactions;
        }
        catch (err) {
            console.log({ err });
            throw err;
        }
    }
}
exports.Relayer = Relayer;
//# sourceMappingURL=relayer.js.map