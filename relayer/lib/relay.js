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
exports.relay = void 0;
const light_sdk_1 = require("light-sdk");
function relay(req) {
    var _a;
    return __awaiter(this, void 0, void 0, function* () {
        const { instructions } = req.body;
        const provider = yield Provider.native(relayerPayer);
        try {
            let ixs = JSON.parse(instructions);
            console.log("PARSED IX:", ixs);
            if (ixs) {
                let tx = "Something went wrong";
                for (let ix in ixs) {
                    let txTmp = yield sendTransaction(ixs[ix], provider);
                    if (txTmp) {
                        console.log("tx ::", txTmp);
                        yield ((_a = this.provider.provider) === null || _a === void 0 ? void 0 : _a.connection.confirmTransaction(txTmp, "confirmed"));
                        tx = txTmp;
                    }
                    else {
                        throw new Error("send transaction failed");
                    }
                }
                return tx;
            }
            else {
                throw new Error("No parameters provided");
            }
        }
        catch (e) {
            console.log(e);
        }
        //TODO: add a check mechanism here await tx.checkBalances();
        console.log("confirmed tx, updating merkletree...");
        yield (0, light_sdk_1.updateMerkleTreeForTest)(provider.provider);
        console.log("merkletree update done. returning 200.");
    });
}
exports.relay = relay;
// module.exports = {
//   relay,
// };
