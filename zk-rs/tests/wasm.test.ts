import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const circomlibjs = require("circomlibjs");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { it } from "mocha";
import {blake2str, poseidon} from "../pkg";
import {BN} from "@coral-xyz/anchor";
const { blake2b } = require("@noble/hashes/blake2b");

describe("Test Account Functional", () => {
    let circomPoseidon: any;

    before(async () => {
        circomPoseidon = await circomlibjs.buildPoseidonOpt();
    });

    it("Test poseidon216", () => {
        const input = new BN([
            216, 137,  85, 159, 239, 194, 107, 138,
            254,  68,  21,  16, 165,  41,  64, 148,
            208, 198, 201,  59, 220, 102, 142,  81,
            49, 251, 174, 183, 183, 182,   4,  32
        ]);

        const expected = [17, 178, 208, 200, 164, 8, 62, 74, 200, 129, 114, 202, 230, 39, 
            233, 241, 8, 80, 197, 195, 65, 90, 170, 52, 234, 36, 44, 9, 174, 211, 102, 225];
        
        const output = new BN(circomPoseidon.F.toString(circomPoseidon([input]))).toArray("be", 32);
        console.log(output);
        assert.equal(output.toString(), expected.toString());
    
    });

    it("Test blake2-simd", () => {
        const input = "foobar";
        const tsBlake = blake2b.create({ dkLen: 32 }).update(input).digest().toString()
        const wasmBlake = blake2str(input, 32).toString();
        assert.equal(tsBlake, wasmBlake);
    })

    it("Test poseidon", () => {
        const inputs = new BN(1).toString();
        const tsHash = new BN(circomPoseidon.F.toString(circomPoseidon([inputs]))).toArray();
        const rsHash = Array.from(poseidon([inputs]));
        assert.equal(tsHash.toString(), rsHash.toString());
    });

    it.skip("Performance blake", () => {
        console.time("blake_ts");
        for (let i = 0; i < 10e6; i++) {
            const tsBlake = blake2b.create({ dkLen: 32 }).update(i.toString()).digest();
        }
        console.timeEnd("blake_ts");

        console.time("blake_rs");
        for (let i = 0; i < 10e6; i++) {
            const rsBlake = blake2str(i.toString(), 32);
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
