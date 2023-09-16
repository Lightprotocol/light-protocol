"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.callDecryptUtxoBytesWorker = void 0;
const tslib_1 = require("tslib");
const web_worker_1 = tslib_1.__importDefault(require("web-worker"));
const decryptUtxoBytesWorker_1 = tslib_1.__importDefault(require("./decryptUtxoBytesWorker"));
// let workerScriptFullPath: string;
// let workerScriptRelativePath = "./decryptUtxoBytesWorker";
let numWorkers = 1;
let workers = [];
if (typeof window === "undefined") {
    // node.js
    //   const os = require("os");
    //   workerScriptFullPath = path.resolve(__dirname, `${workerScriptRelativePath}`);
    //   numWorkers = os.cpus().length - 1;
    //   console.log("os.cpus().length", os.cpus().length);
}
else {
    // browser
    //   workerScriptFullPath = workerScriptRelativePath; // TODO: might have to adjust
    numWorkers = navigator.hardwareConcurrency - 1;
}
function stringToBase64(str) {
    //@ts-ignore
    if (process.browser) {
        return globalThis.btoa(str);
    }
    else {
        return Buffer.from(str).toString("base64");
    }
}
// Write the worker script to a temporary file
const threadSource = stringToBase64("(" + decryptUtxoBytesWorker_1.default.toString() + ")(self)");
const workerSource = "data:application/javascript;base64," + threadSource;
workers = Array.from({ length: numWorkers }, () => new web_worker_1.default(workerSource));
// workers = Array.from(
//   { length: numWorkers },
//   () =>
//     new Worker(
//       path.resolve(__dirname, "./../../lib/workers/decryptUtxoBytesWorker.js"),
//       {
//         type: "module",
//       },
//     ),
// );
async function callDecryptUtxoBytesWorker(params) {
    // Split the tasks among workers
    const tasks = chunkArray(params.encBytesArray, numWorkers);
    // Create promises for each worker
    const promises = tasks.map((task, index) => {
        return new Promise((resolve, reject) => {
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
            transferList.push(params.aesSecret.buffer);
            //   transferList.push(
            //     params.asymSecret ? params.asymSecret.buffer : undefined,
            //   );
            console.log("TRANSFER LIST: ", transferList);
            // Post message to worker
            worker.postMessage({ ...params, encBytesArray: task }, transferList);
        });
    });
    // Wait for all workers to finish and collect results
    const results = (await Promise.all(promises)).flat();
    return results;
}
exports.callDecryptUtxoBytesWorker = callDecryptUtxoBytesWorker;
function chunkArray(array, size) {
    var results = [];
    while (array.length) {
        results.push(array.splice(0, size));
    }
    return results;
}
//# sourceMappingURL=workerUtils.js.map