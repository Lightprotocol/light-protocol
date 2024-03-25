export type { LightWasm, LightWasmCreator, HasherLoadOptions } from "./model.js";
export { WasmFactory, hasSimd as hasWasmSimd } from "./wasm.js";
import wasm from "./wasm/hasher_bg.wasm";
import wasmSimd from "./wasm-simd/hasher_wasm_simd_bg.wasm";
import { setWasmInit, setWasmSimdInit } from "./wasm.js";

// @ts-ignore
setWasmInit(() => wasm());
// @ts-ignore
setWasmSimdInit(() => wasmSimd());
