"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.callDecryptUtxoBytesWorker = void 0;
const web_worker_1 = __importDefault(require("web-worker"));
let workerScriptFullPath;
// let workerScriptRelativePath = "./../../lib/workers/decryptUtxoBytesWorker.js";
let workerScriptRelativePath = "./decryptUtxoBytesWorker";
let numWorkers = 1;
let workers = [];
const path = require("path");
if (typeof window === "undefined") {
    // node.js
    const os = require("os");
    workerScriptFullPath = path.resolve(__dirname, `${workerScriptRelativePath}`);
    //   numWorkers = os.cpus().length - 1;
    console.log("os.cpus().length", os.cpus().length);
}
else {
    // browser
    workerScriptFullPath = workerScriptRelativePath; // TODO: might have to adjust
    numWorkers = navigator.hardwareConcurrency - 1;
    // Create workers
}
workers = Array.from({ length: numWorkers }, () => 
// new Worker(new URL("./decryptUtxoBytesWorker.js", import.meta.url), {
new web_worker_1.default(path.resolve(__dirname, "./../../lib/workers/decryptUtxoBytesWorker.js")));
console.log("workerScriptFullPath:", workerScriptFullPath, "relative:", workerScriptRelativePath);
console.log("numWorkers", numWorkers);
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
                    transferList.push(encByte.leftLeaf.buffer);
                    if (encByte.encBytes instanceof Buffer) {
                        transferList.push(encByte.encBytes.buffer);
                    }
                }
            }
            // passing this to determine in worker whether to use aes or asymmetric decryption. TOOD: find a more elegant approach
            transferList.push(params.aesSecret ? params.aesSecret.buffer : undefined);
            transferList.push(params.asymSecret ? params.asymSecret.buffer : undefined);
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