import Worker from "web-worker";
import { UtxoBatch } from "wallet";
import { UtxoBytes } from "utxo";
let workerScriptFullPath: string;
// let workerScriptRelativePath = "./../../lib/workers/decryptUtxoBytesWorker.js";
let workerScriptRelativePath = "./decryptUtxoBytesWorker";
let numWorkers = 1;

if (typeof window === "undefined") {
  // node.js
  const path = require("path");
  const os = require("os");
  workerScriptFullPath = path.resolve(__dirname, `${workerScriptRelativePath}`);
  //   numWorkers = os.cpus().length - 1;
  console.log("os.cpus().length", os.cpus().length);
} else {
  // browser
  workerScriptFullPath = workerScriptRelativePath; // TODO: might have to adjust
  numWorkers = navigator.hardwareConcurrency - 1;
  // Create workers
}

const workers = Array.from(
  { length: numWorkers },
  () =>
    new Worker(new URL("./decryptUtxoBytesWorker.ts", import.meta.url), {
      type: "module",
    }),
);

console.log(
  "workerScriptFullPath:",
  workerScriptFullPath,
  "relative:",
  workerScriptRelativePath,
);
console.log("numWorkers", numWorkers);

export async function callDecryptUtxoBytesWorker(params: {
  encBytesArray: UtxoBatch[];
  compressed: boolean;
  aesSecret: Uint8Array | undefined;
  asymSecret: Uint8Array | undefined;
  merkleTreePdaPublicKeyString: string;
}): Promise<UtxoBytes[]> {
  // Split the tasks among workers
  const tasks = chunkArray(params.encBytesArray, numWorkers);

  // Create promises for each worker
  const promises = tasks.map((task, index) => {
    return new Promise<UtxoBytes>((resolve, reject) => {
      const worker = workers[index];

      worker.onmessage = (e) => {
        resolve(e.data);
      };
      worker.onerror = reject;

      // Prepare transfer list
      let transferList = [];
      for (let encBytesArray of task) {
        for (let encByte of encBytesArray.encryptedUtxos) {
          transferList.push(encByte.leftLeaf.buffer);
          if (encByte.encBytes instanceof Buffer) {
            transferList.push(encByte.encBytes.buffer);
          }
        }
      }
      // passing this to determine in worker whether to use aes or asymmetric decryption. TOOD: find a more elegant approach
      transferList.push(params.aesSecret ? params.aesSecret.buffer : undefined);
      transferList.push(
        params.asymSecret ? params.asymSecret.buffer : undefined,
      );

      // Post message to worker
      worker.postMessage({ ...params, encBytesArray: task }, transferList);
    });
  });

  // Wait for all workers to finish and collect results
  const results: UtxoBytes[] = (await Promise.all(promises)).flat();
  return results;
}

function chunkArray(array: any[], size: number) {
  var results = [];
  while (array.length) {
    results.push(array.splice(0, size));
  }
  return results;
}
