import {blake2str, poseidon} from "../pkg/light_wasm";
import {BN} from "@coral-xyz/anchor";
import { blake2b } from "@noble/hashes/blake2b";
import * as circomlibjs from "circomlibjs";

import { Bench } from 'tinybench';
const bench = new Bench({ time: 5000 });
const circomPoseidon = await circomlibjs.buildPoseidonOpt();

bench
.add('wasm poseidon', () => {
    poseidon(["1"]);
})
.add('circom poseidon', async () => {
    circomPoseidon(["1"]);
})

await bench.run();

console.table(bench.table());