import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const circomlibjs = require("circomlibjs");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { it } from "mocha";
import {blake2str } from "light-wasm";
import {BN} from "@coral-xyz/anchor";
import { Poseidon } from "../src";
const { blake2b } = require("@noble/hashes/blake2b");
import { Scalar } from "ffjavascript";
describe("Test Account Functional", () => {
    let circomPoseidon: any;
    let poseidon: Poseidon;
    before(async () => {
        circomPoseidon = await circomlibjs.buildPoseidonOpt();
        poseidon = await Poseidon.getInstance();
    });

    it("Test poseidon216", () => {
        let input = new BN([
            216, 137,  85, 159, 239, 194, 107, 138,
            254,  68,  21,  16, 165,  41,  64, 148,
            208, 198, 201,  59, 220, 102, 142,  81,
            49, 251, 174, 183, 183, 182,   4,  32
        ]);

        input = new BN(new Uint8Array(input.toArray()).slice(1, 32), undefined, "be");
        const expected = [39,11,135,4,126,124,21,55,122,162,99,228,196,251,107,128,181,191,102,183,35,64,122,163,42,155,219,100,30,89,203,0];
        
        const circomOutput = new BN(circomPoseidon.F.toString(circomPoseidon([input]))).toArray("be", 32);
        assert.equal(circomOutput.toString(), expected.toString());

        const wasmOutput = new BN(poseidon.hash([input])).toArray("be", 32);
        assert.equal(wasmOutput.toString(), expected.toString());
    });

    it("Test blake2-simd", () => {
        const input = "foobar";
        const tsBlake = blake2b.create({ dkLen: 32 }).update(input).digest().toString()
        const wasmBlake = blake2str(input, 32).toString();
        assert.equal(tsBlake, wasmBlake);
    })

    it("Test Poseidon", () => {
        const inputs = new BN(1).toString();
        const tsHash = new BN(circomPoseidon.F.toString(circomPoseidon([inputs]))).toArray();
        const rsHash = Array.from(poseidon.hash([inputs]));
        assert.equal(tsHash.toString(), rsHash.toString());
    });

});
