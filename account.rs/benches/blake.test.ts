import {blake2str, poseidon} from "light-wasm";
import {BN} from "@coral-xyz/anchor";
import { blake2b } from "@noble/hashes/blake2b";

import { Bench } from 'tinybench';
const bench = new Bench({ time: 1000 });

bench
.add('wasm blake', () => {
    blake2str("", 32);
})
.add('@noble/hashes/blake2b', async () => {
    blake2b.create({ dkLen: 32 }).update("").digest();
})

await bench.run();

console.table(bench.table());