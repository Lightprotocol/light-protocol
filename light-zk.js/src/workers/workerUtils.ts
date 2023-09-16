import Worker from "web-worker";
import { UtxoBatch } from "wallet";
import { UtxoBytes } from "utxo";

// let workerScriptFullPath: string;
// let workerScriptRelativePath = "./decryptUtxoBytesWorker";
let numWorkers = 1;
let workers: Worker[] = [];

if (typeof window === "undefined") {
  // node.js
  //   const os = require("os");
  //   workerScriptFullPath = path.resolve(__dirname, `${workerScriptRelativePath}`);
  //   numWorkers = os.cpus().length - 1;
  //   console.log("os.cpus().length", os.cpus().length);
} else {
  // browser
  //   workerScriptFullPath = workerScriptRelativePath; // TODO: might have to adjust
  //   numWorkers = navigator.hardwareConcurrency - 1;
}

// Write the worker script to a temporary file

workers = Array.from(
  { length: numWorkers },
  () =>
    new Worker(
      //   path.resolve(__dirname, "./../../lib/workers/decryptUtxoBytesWorker.js"),
      new URL("./decryptUtxoBytesWorker.js", import.meta.url),
      {
        type: "module",
      },
    ),
);

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
          transferList.push(Uint8Array.from(encByte.leftLeaf).buffer);
          if (encByte.encBytes instanceof Buffer) {
            transferList.push(Uint8Array.from(encByte.encBytes).buffer);
          }
        }
      }
      // passing this to determine in worker whether to use aes or asymmetric decryption. TOOD: find a more elegant approach
      //   transferList.push(params.aesSecret ? params.aesSecret.buffer : undefined);
      transferList.push(params.aesSecret!.buffer);
      //   transferList.push(
      //     params.asymSecret ? params.asymSecret.buffer : undefined,
      //   );
      console.log("TRANSFER LIST: ", transferList);
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
