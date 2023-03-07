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
exports.sendTransaction = void 0;
const web3_js_1 = require("@solana/web3.js");
const light_sdk_1 = require("light-sdk");
function sendTransaction(ix, provider) {
    return __awaiter(this, void 0, void 0, function* () {
        if (!provider.provider)
            throw new Error("no provider set");
        const recentBlockhash = (yield provider.provider.connection.getRecentBlockhash("confirmed")).blockhash;
        const txMsg = new web3_js_1.TransactionMessage({
            payerKey: provider.nodeWallet.publicKey,
            instructions: [
                web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
                ix,
            ],
            recentBlockhash: recentBlockhash,
        });
        const lookupTableAccount = yield provider.provider.connection.getAccountInfo(provider.lookUpTable, "confirmed");
        const unpackedLookupTableAccount = web3_js_1.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
        const compiledTx = txMsg.compileToV0Message([
            {
                state: unpackedLookupTableAccount,
                key: provider.lookUpTable,
                isActive: () => {
                    return true;
                },
            },
        ]);
        compiledTx.addressTableLookups[0].accountKey = provider.lookUpTable;
        var tx = new web3_js_1.VersionedTransaction(compiledTx);
        let retries = 3;
        let res;
        while (retries > 0) {
            tx.sign([provider.nodeWallet]);
            try {
                let serializedTx = tx.serialize();
                console.log("tx: ");
                res = yield provider.provider.connection.sendRawTransaction(serializedTx, light_sdk_1.confirmConfig);
                retries = 0;
                // console.log(res);
            }
            catch (e) {
                retries--;
                if (retries == 0 || e.logs !== undefined) {
                    console.log(e);
                    return e;
                }
            }
        }
        return res;
    });
}
exports.sendTransaction = sendTransaction;
