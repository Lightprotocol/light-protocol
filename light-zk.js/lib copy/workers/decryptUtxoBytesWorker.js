"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
console.log("hi from worker file");
// custom implementation like ffjavascript
function work() {
    // const { UtxoBatch } = require("../wallet");
    if (self) {
        console.log("self true");
        self.onmessage = function (e) {
            let params;
            if (e.data) {
                params = e.data;
            }
            else {
                params = e;
            }
            bulkDecryptUtxoBytes(params.encBytesArray, params.compressed, new Uint8Array(params.aesSecret), new Uint8Array(params.asymSecret), params.merkleTreePdaPublicKeyString).then((result) => {
                // Construct UtxoBytes objects
                let utxoBytesArray = result.map((item) => ({
                    bytes: item.bytes ? Buffer.from(item.bytes) : null,
                    leftLeaf: new Uint8Array(item.leftLeaf),
                    index: item.index,
                }));
                // Prepare transfer list
                let transferList = [];
                for (let item of utxoBytesArray) {
                    if (item.bytes) {
                        transferList.push(item.bytes.buffer);
                    }
                    transferList.push(item.leftLeaf.buffer);
                }
                // Post message back to the main thread
                self.postMessage(utxoBytesArray, "*", transferList);
            });
        };
    }
    /**
     *
     *
     * @description Decrypts utxos In bulk
     * @param {Uint8Array} encBytes
     * @param {boolean} compressed
     * @param {Uint8Array} commitment
     * @param {Uint8Array} aesSecret
     * @param {Uint8Array} asymSecret
     * @returns {Promise<Uint8Array>}
     */
    async function bulkDecryptUtxoBytes(encBytesArray, compressed, aesSecret, asymSecret, merkleTreePdaPublicKeyString) {
        const web3 = await Promise.resolve().then(() => __importStar(require("@solana/web3.js")));
        const utxo = await Promise.resolve().then(() => __importStar(require("../utxo")));
        let merkleTreePdaPublicKey = new web3.PublicKey(merkleTreePdaPublicKeyString);
        console.log("BULK DECRYPTING");
        let promises = [];
        for (const encBytes of encBytesArray) {
            for (const encByte of encBytes.encryptedUtxos) {
                let commitment = encByte.commitment;
                promises.push(utxo.Utxo.fastDecrypt({
                    merkleTreePdaPublicKey,
                    compressed,
                    commitment,
                    encBytes: new Uint8Array(encByte.encBytes),
                    aesSecret,
                    asymSecret,
                }).then((bytes) => ({
                    // We need to access leftLeaf when modifying the balance in the mainThread
                    // TODO: Instead, we could pass leafLeft as param and resolve directly to it.
                    bytes,
                    leftLeaf: encByte.leftLeaf,
                    index: encByte.index,
                })));
            }
        }
        let results = (await Promise.all(promises)).filter((res) => res.bytes != null);
        console.log("results", results);
        return results;
    }
    /// For use inside workers where Account instance non accessible (fastDecrypt)
}
exports.default = work;
// addEventListener("message", async (e) => {
//   let params = e.data;
//   bulkDecryptUtxoBytes(
//     params.encBytesArray,
//     params.compressed,
//     new Uint8Array(params.aesSecret),
//     new Uint8Array(params.asymSecret),
//     params.merkleTreePdaPublicKeyString,
//   ).then((result) => {
//     // Construct UtxoBytes objects
//     let utxoBytesArray = result.map((item) => ({
//       bytes: item.bytes ? Buffer.from(item.bytes) : null,
//       leftLeaf: new Uint8Array(item.leftLeaf),
//       index: item.index,
//     }));
//     // Prepare transfer list
//     let transferList = [];
//     for (let item of utxoBytesArray) {
//       if (item.bytes) {
//         transferList.push(item.bytes.buffer);
//       }
//       transferList.push(item.leftLeaf.buffer);
//     }
//     // Post message back to the main thread
//     postMessage(utxoBytesArray, "*", transferList);
//   });
// });
//# sourceMappingURL=decryptUtxoBytesWorker.js.map