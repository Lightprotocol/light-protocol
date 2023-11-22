export { blake } from "./blake";
export { Poseidon } from "./poseidon";

import * as wasm from "./wasm/light_wasm";

export async function loadWasm() {
    await wasm.default();
}
