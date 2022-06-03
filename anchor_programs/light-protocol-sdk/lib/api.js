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
exports.withdraw = void 0;
const wallet_adapter_base_1 = require("@solana/wallet-adapter-base");
// maybe own file for nacl and imprt when needed?
const tweetnacl_1 = require("tweetnacl");
const constants_1 = require("./constants");
const encryptMessage_1 = require("./utils/encryptMessage");
const enums_1 = require("./enums");
const newNonce = () => (0, tweetnacl_1.randomBytes)(tweetnacl_1.box.nonceLength);
const newKeypair = () => tweetnacl_1.box.keyPair();
const axios = require('axios').default;
const withdraw = function ({ publicKey, inputBytes, proofBytes, extDataBytes, recipient, action, amount, encryptionKeypair, uuid, timeout, }) {
    return __awaiter(this, void 0, void 0, function* () {
        console.time('Withdrawal');
        if (!publicKey) {
            throw new wallet_adapter_base_1.WalletNotConnectedError();
        }
        /// Make http request to relayer
        // TODO This function does nothing it is the same before and after
        const extData = [];
        extDataBytes.map((x, i) => {
            extData[i] = x;
        });
        const onetimeKeypair = newKeypair();
        const nonce = newNonce();
        const ciphertext = (0, encryptMessage_1.encryptMessage)(publicKey.toBytes(), nonce, encryptionKeypair, onetimeKeypair);
        const payload = {
            input: inputBytes,
            proof: proofBytes,
            extData: extData,
            recipientTokenPdaSecretKey: null,
            recipient: recipient,
            action: enums_1.Action[action],
            amount: amount,
            hashedPubkey: ciphertext,
            throwawayPubkey: onetimeKeypair.publicKey,
            nonce: nonce,
            uuid: uuid,
            timeout: timeout,
        };
        const res = yield axios.post(`${constants_1.REACT_APP_RELAYER_URL}/w-unshield`, payload);
        console.log('/post response:', res);
        if (res.status == 500) {
            // https://axios-http.com/docs/res_schema
            throw new Error(res.data.message);
        }
        if (res.status === 574) {
            throw new Error('sessionTimeoutError');
        }
        if (res.status === 572) {
            throw new Error('programBusyError');
        }
        console.timeEnd('Withdrawal took:');
        return res;
    });
};
exports.withdraw = withdraw;
