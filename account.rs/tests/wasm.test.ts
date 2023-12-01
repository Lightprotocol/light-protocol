import {beforeAll} from "vitest";
const { blake2b } = require("@noble/hashes/blake2b");
import * as circomlibjs from "circomlibjs";
import { AccountHash, WasmHash, hasWasmSimd } from "..";
import {BN} from "@coral-xyz/anchor";

function isNode() {
    return (
        Object.prototype.toString.call(
            typeof process !== "undefined" ? process : 0
        ) === "[object process]"
    );
}

describe("Test Account Functional", () => {

    let poseidon: any;

    beforeEach(() => {
        WasmHash.resetModule();
        AccountHash.resetModule();
    });

    beforeAll(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
    });

    it("choose hash implementation depending on platform", () => {
        if (isNode()) {
            expect(WasmHash.name).toEqual("WasmHash");
        } else {
            expect(WasmHash.name).toContain("WasmAccountHash");
        }
    });

    it("Test poseidon216", async () => {
        let input = new BN([
            216, 137,  85, 159, 239, 194, 107, 138,
            254,  68,  21,  16, 165,  41,  64, 148,
            208, 198, 201,  59, 220, 102, 142,  81,
            49, 251, 174, 183, 183, 182,   4,  32
        ]);
        input = new BN(new Uint8Array(input.toArray()).slice(1, 32), undefined, "be");

        const expected = [39,11,135,4,126,124,21,55,122,162,99,228,196,251,107,128,181,191,102,183,35,64,122,163,42,155,219,100,30,89,203,0];

        const circomOutput = new BN(poseidon.F.toString(poseidon([input]))).toArray("be", 32);
        assert.equal(circomOutput.toString(), expected.toString());

        const mod = await WasmHash.loadModule();
        const hash = mod.create();
        const wasmOutput = new BN(hash.poseidonHash([input])).toArray("be", 32);

        assert.equal(wasmOutput.toString(), expected.toString());
    });

    it("Test blake2-simd", async () => {
        const input = "foobar";
        const tsBlake = blake2b.create({ dkLen: 32 }).update(input).digest().toString()

        const mod = await WasmHash.loadModule();
        const hash = mod.create();
        const wasmBlake = hash.blakeHash(input, 32).toString();
        assert.equal(tsBlake, wasmBlake);
    })

    it("Test Poseidon", async () => {
        const inputs = new BN(1).toString();
        const tsHash = new BN(poseidon.F.toString(poseidon([inputs]))).toArray();

        const mod = await WasmHash.loadModule();
        const hash = mod.create();
        const rsHash = hash.poseidonHash([inputs]);

        assert.equal(tsHash.toString(), rsHash.toString());
    });

});
