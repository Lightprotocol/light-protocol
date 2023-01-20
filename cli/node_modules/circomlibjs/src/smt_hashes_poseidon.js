import { buildPoseidon } from "./poseidon_wasm.js";
import { getCurveFromName }  from "ffjavascript";


export default async function getHashes() {
    const bn128 = await getCurveFromName("bn128", true);
    const poseidon = await buildPoseidon();
    return {
        hash0: function (left, right) {
            return poseidon([left, right]);
        },
        hash1: function(key, value) {
            return poseidon([key, value, bn128.Fr.one]);
        },
        F: bn128.Fr
    }
}
