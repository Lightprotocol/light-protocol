import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const circomlibjs = require("circomlibjs");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { it } from "mocha";
import {add, blake2_hash, poseidon} from "light-wasm";
import {BN} from "@coral-xyz/anchor";
import {bs58} from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {Account, BN_0, BN_1} from "../lib";
import nacl from "tweetnacl";
const { blake2b } = require("@noble/hashes/blake2b");

describe("Test Account Functional", () => {
    let circomPoseidon: any;

    before(async () => {
        circomPoseidon = await circomlibjs.buildPoseidonOpt();
    });

    it("Test add/wasm", () => {
        assert.equal(add(1, 2), 3);
    });

    it("Test blake2-simd", () => {
        const input = "foobar";
        const tsBlake = blake2b.create({ dkLen: 32 }).update(input).digest().toString()
        const wasmBlake = blake2_hash(input, 32).toString();
        assert.equal(tsBlake, wasmBlake);
    })

    it("Test poseidon", () => {
        const inputs = BN_1.toString();
        const tsHash = new BN(circomPoseidon.F.toString(circomPoseidon([inputs]))).toArray();
        const rsHash = Array.from(poseidon([inputs]));
        assert.equal(tsHash.toString(), rsHash.toString());
    });

    it.only("Test poseidon216", () => {
        const input = new BN([
            216, 137,  85, 159, 239, 194, 107, 138,
            254,  68,  21,  16, 165,  41,  64, 148,
            208, 198, 201,  59, 220, 102, 142,  81,
            49, 251, 174, 183, 183, 182,   4,  32
        ]).toString();
        console.log(input);
        const output = new BN(circomPoseidon.F.toString(circomPoseidon([input])));
        console.log(output.toArray("be", 32));

        //
        // const hash_of_1 = [
        //     41, 23, 97, 0, 234, 169, 98, 189, 193, 254, 108, 101, 77, 106, 60, 19, 14, 150, 164,
        //     209, 22, 139, 51, 132, 139, 137, 125, 197, 2, 130, 1, 51,
        // ];
        //
        // const expected_hash_of_1 = [
        //     41, 23, 97, 0, 234, 169, 98, 189, 193, 254, 108, 101, 77, 106, 60, 19, 14, 150, 164,
        //     209, 22, 139, 51, 132, 139, 137, 125, 197, 2, 130, 1, 51,
        // ];
        //
        // const input_of_1 = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        // const inputs_of_1_BN = new BN(input_of_1);
        // const poseidon_of_1 = new BN(circomPoseidon.F.toString(circomPoseidon([inputs_of_1_BN])));
        // console.log("poseidon_of_1: ", poseidon_of_1.toArray().toString());
        // assert.equal(expected_hash_of_1.toString(), poseidon_of_1.toArray().toString());
    });


    it.skip("Performance blake", () => {
        console.time("blake_ts");
        for (let i = 0; i < 10e6; i++) {
            const tsBlake = blake2b.create({ dkLen: 32 }).update(i.toString()).digest();
        }
        console.timeEnd("blake_ts");

        console.time("blake_rs");
        for (let i = 0; i < 10e6; i++) {
            const rsBlake = blake2_hash(i.toString());
        }
        console.timeEnd("blake_rs");
    });

    it.skip("Performance poseidon", () => {
        console.time("poseidon_ts");
        for (let i = 0; i < 10e5; i++) {
            const inputs = new BN(i).toString();
            const tsPoseidonResult = circomPoseidon.F.toString(circomPoseidon([inputs]));
        }
        console.timeEnd("poseidon_ts");

        console.time("poseidon_rs");
        for (let i = 0; i < 10e5; i++) {
            const inputs = new BN(i).toString();
            const rsHash = poseidon([inputs]);
        }
        console.timeEnd("poseidon_rs");
    });

});
