"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.LightSDK = void 0;
const api_1 = require("./api");
const constants_1 = require("./constants");
const createOutputUtxo_1 = require("./createOutputUtxo");
const enums_1 = require("./enums");
const prepareTransaction_1 = require("./prepareTransaction");
const buildMerkelTree_1 = require("./buildMerkelTree");
const proof_1 = require("./proof");
const checkAddress_1 = require("./checkAddress");
const errors_1 = require("./errors");
class LightSDK {
    unshield(recipient, amount, handle) {
        return __awaiter(this, void 0, void 0, function* () {
            if (!(0, checkAddress_1.checkAddress)(recipient))
                return errors_1.invalidAddressError;
            if (amount <= 0)
                return errors_1.insufficientAmountError;
            const token = enums_1.Token.SOL;
            const left = 0;
            const top = 0;
            const width = 500;
            const height = 700;
            const positionAndSize = `left=${left},top=${top},width=${width},height=${height},popup,resizable,scrollbars,titlebar=0,toolbar=0,location=0,status=0`;
            const popupWindow = window.open(`${constants_1.LIGHTSHIELD_WIDGET}?handle=${handle}&token=${enums_1.Token[token]}&recipient=${recipient}&amount=${amount}`, '_blank', positionAndSize);
            if (popupWindow == null) {
                return errors_1.popupError;
            }
            let result = yield new Promise((resolve, reject) => {
                window.onmessage = (event) => {
                    if (event.origin !== 'https://widget.lightprotocol.com') {
                        reject('Wrong origin.');
                    }
                    else {
                        if (event.data.amount && event.data.recipient) {
                            if (event.data.recipient === recipient &&
                                Number(event.data.amount) === Number(amount)) {
                                if (event.data.unshieldSuccess) {
                                    resolve(true);
                                }
                                else {
                                    reject('Unshielding failed.');
                                }
                            }
                            else {
                                reject('Wrong recipient or amount.');
                            }
                        }
                        else {
                            console.log('Ignoring external event.');
                        }
                    }
                };
            });
            return result;
        });
    }
    /**
     * Prepare and send unshielding
     *
     * @param publicKey publicKey of [placeholder]
     * @param recipient recipient of the unshielding
     * @param amount amount to be unshielded
     * @param token token used for unshielding
     * @param encryptionPubkey encryptionPubkey used for encryption
     * @param inputUtxos utxos to pay with
     * @param relayerFee fee for the relayer
     * @param connection RPC connection
     * @param shieldedKeypair shieldedKeypair
     * @param timeout timeout timestamp in ms
     */
    prepareAndSendUnshield(publicKey, recipient, amount, token, encryptionKeypair, inputUtxos, relayerFee, connection, shieldedKeypair, uuid, timeout) {
        return __awaiter(this, void 0, void 0, function* () {
            const outputUtxo = (0, createOutputUtxo_1.createOutputUtxo)(inputUtxos, amount, shieldedKeypair, relayerFee);
            let transaction;
            try {
                transaction = (0, prepareTransaction_1.prepareTransaction)(inputUtxos, [outputUtxo], relayerFee, enums_1.Action.WITHDRAWAL);
            }
            catch (err) {
                console.log(err);
                throw new Error('PrepareTransaction failed.');
            }
            let merkelTree;
            try {
                merkelTree = yield (0, buildMerkelTree_1.buildMerkelTree)(connection);
            }
            catch (err) {
                console.log(err);
                throw new Error('BuildMerkelTree failed.');
            }
            // console.log("transaction.inputUtxos", transaction.inputUtxos)
            // console.log("transaction.outputUtxos", transaction.outputUtxos)
            // console.log("transaction.externalAmountBigNumber", transaction.externalAmountBigNumber)
            // console.log("relayerFee", relayerFee,)
            // console.log("recipien", recipient)
            // console.log("RELAYER_ADDRESS", RELAYER_ADDRESS)
            // console.log("Action.withdrawal", Action.withdrawal,)
            // console.log("encryptionKeypair", encryptionKeypair)
            const { data } = yield (0, proof_1.getProof)(transaction.inputUtxos, transaction.outputUtxos, merkelTree, transaction.externalAmountBigNumber, relayerFee, recipient, constants_1.RELAYER_ADDRESS, enums_1.Action.WITHDRAWAL, encryptionKeypair);
            console.log('Prooof return data', data);
            const { publicInputsBytes, proofBytes, extDataBytes } = data;
            try {
                let res = yield (0, api_1.withdraw)({
                    publicKey: publicKey,
                    inputBytes: publicInputsBytes,
                    proofBytes: proofBytes,
                    extDataBytes: extDataBytes,
                    recipient: recipient,
                    action: enums_1.Action.WITHDRAWAL,
                    amount: amount,
                    encryptionKeypair: encryptionKeypair,
                    uuid: uuid,
                    timeout: timeout,
                });
                console.log(res);
                let resAmount = res.data.amount;
                let resRecipient = res.data.recipient;
                console.log(`resAmount: ${resAmount} - amount: ${amount} - resRecipient: ${resRecipient} - recipient: ${recipient}`);
                // Should never happen
                if (resAmount !== amount || resRecipient !== recipient) {
                    throw new Error('Invalid recipient or amount');
                }
                return { recipient: resRecipient, amount: resAmount };
            }
            catch (err) {
                console.log('error @withdraw: ', err);
                throw new Error(err === null || err === void 0 ? void 0 : err.message);
            }
        });
    }
}
exports.LightSDK = LightSDK;
