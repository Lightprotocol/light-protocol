const wasm = require("./light_wasm.js");

let initializedWasmModule;

/**
 * Load WASM module
 * @returns {Promise<void>}
 */
const initializeWasm = async () => {
    if (!initializedWasmModule) {
        initializedWasmModule = await wasm.default();
    }
};

/**
 * Returns true if WASM module loaded, false otherwise
 * @returns {boolean}
 */
const isWasmInitialized = () => {
    return initializedWasmModule !== undefined
};

/**
 * Throws an error if WASM module is not loaded.
 */
const requireWasmInitialized = () => {
    if (!isWasmInitialized()) {
        throw new Error('WASM module is not initialized. Call `initialize` first and wait for it to resolve')
    }
};

module.exports = {
    wasm, initializeWasm, isWasmInitialized, requireWasmInitialized
}
