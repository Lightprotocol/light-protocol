export type { IHash, HashCreator, AccountLoadOptions } from "./model.js";
export { WasmHash, hasSimd as hasWasmSimd } from "./wasm.js";
import wasm from "./wasm/account_wasm_bg.wasm";
import wasmSimd from "./wasm-simd/account_wasm_simd_bg.wasm";
import { setWasmInit, setWasmSimdInit } from "./wasm.js";

// @ts-ignore
setWasmInit(() => wasm());
// @ts-ignore
setWasmSimdInit(() => wasmSimd());
