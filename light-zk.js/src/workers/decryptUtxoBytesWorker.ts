import { PublicKey } from "@solana/web3.js";
import { Utxo, UtxoBytes } from "../utxo";
import { UtxoBatch } from "../wallet";

console.log("hi from worker file");

addEventListener("message", async (e) => {
  console.log("received message from main thread");
  let params = e.data;
  bulkDecryptUtxoBytes(
    params.encBytesArray,
    params.compressed,
    new Uint8Array(params.aesSecret),
    new Uint8Array(params.asymSecret),
    params.merkleTreePdaPublicKeyString,
  ).then((result) => {
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
    postMessage(utxoBytesArray, "*", transferList);
  });
});

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
