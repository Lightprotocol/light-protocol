import * as workerpool from "workerpool";
import { PublicKey } from "@solana/web3.js";
import { Utxo, UtxoBytes } from "../utxo";
import { UtxoBatch } from "../wallet";

console.log("hi from worker file");
// load workerpool
// if (typeof importScripts === "function") {
//   // web worker
//   importScripts("workerpool");
// } else {
//   // node.js
// //   var workerpool = require("../../dist/workerpool.js");
// }
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
async function bulkDecryptUtxoBytes(
  encBytesArray: UtxoBatch[],
  compressed: boolean,
  aesSecret: Uint8Array,
  asymSecret: Uint8Array,
  merkleTreePdaPublicKeyString: string,
): Promise<UtxoBytes[]> {
  let merkleTreePdaPublicKey = new PublicKey(merkleTreePdaPublicKeyString);

  let promises = [];
  for (const encBytes of encBytesArray) {
    for (const encByte of encBytes.encryptedUtxos) {
      let commitment = encByte.commitment;

      promises.push(
        Utxo.fastDecrypt({
          merkleTreePdaPublicKey,
          compressed,
          commitment,
          encBytes: new Uint8Array(encByte.encBytes!),
          aesSecret,
          asymSecret,
        }).then((bytes) => ({
          // We need to access leftLeaf when modifying the balance in the mainThread
          // TODO: Instead, we could pass leafLeft as param and resolve directly to it.
          bytes,
          leftLeaf: encByte.leftLeaf,
          index: encByte.index,
        })),
      );
    }
  }
  let results = (await Promise.all(promises)).filter(
    (res) => res.bytes != null,
  );
  console.log("results", results);
  return results;
}

// create a worker and register public functions
workerpool.worker({
  bulkDecryptUtxoBytes: bulkDecryptUtxoBytes,
});
