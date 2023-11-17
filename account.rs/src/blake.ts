import {blake2, blake2str} from "light-wasm";
import { featureFlags } from "./config";
import {blake2b} from "@noble/hashes/blake2b";

export function blake(input: string|Uint8Array, hashLength: number = 32): Uint8Array {
    if (featureFlags.wasmPoseidon) {
        if (typeof input === 'string') {
            return blake2str(input, hashLength);
        }
        else {
            return blake2(input, hashLength);
        }
    } else {
        const b2params = { dkLen: hashLength };
        return blake2b.create(b2params).update(input).digest();
    }
}
