import {beforeAll} from "vitest";
const { blake2b } = require("@noble/hashes/blake2b");
import * as circomlibjs from "circomlibjs";
import { AccountHasher, WasmHasher } from "..";
import {BN} from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Account as AccountWasm } from "../src/main/wasm/account_wasm";

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
        WasmHasher.resetModule();
        AccountHasher.resetModule();
    });

    beforeAll(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
    });

    it("choose hash implementation depending on platform", () => {
        if (isNode()) {
            expect(WasmHasher.name).toEqual("WasmHasher");
        } else {
            expect(WasmHasher.name).toContain("WasmAccountHash");
        }
    });

    it("test account", async () => {
        const seed32 = (): string => {
            return bs58.encode(new Uint8Array(32).fill(1));
        };

        const mod = await WasmHasher.loadModule();
        const hash = mod.create();

        const account: AccountWasm = hash.account(seed32());
        const account2: AccountWasm = hash.account(seed32());
        expect(account2.getPrivateKey()).toEqual(account.getPrivateKey());
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

        const mod = await WasmHasher.loadModule();
        const hash = mod.create();
        const wasmOutput = new BN(hash.poseidonHash([input])).toArray("be", 32);

        assert.equal(wasmOutput.toString(), expected.toString());
    });

    it("Test blake2-simd", async () => {
        const input = "foobar";
        const tsBlake = blake2b.create({ dkLen: 32 }).update(input).digest().toString()

        const mod = await WasmHasher.loadModule();
        const hash = mod.create();
        const wasmBlake = hash.blakeHash(input, 32).toString();
        assert.equal(tsBlake, wasmBlake);
    })

    it("Test Poseidon", async () => {
        const inputs = new BN(1).toString();
        const tsHash = new BN(poseidon.F.toString(poseidon([inputs]))).toArray();

        const mod = await WasmHasher.loadModule();
        const hash = mod.create();
        const rsHash = hash.poseidonHash([inputs]);

        assert.equal(tsHash.toString(), rsHash.toString());
    });

    it("Test Poseidon 1..12", async() => {
        let TEST_CASES = [
            [
                41, 23, 97, 0, 234, 169, 98, 189, 193, 254, 108, 101, 77, 106, 60, 19, 14, 150, 164, 209,
                22, 139, 51, 132, 139, 137, 125, 197, 2, 130, 1, 51,
            ],
            [
                0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167, 138,
                203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129,
            ],
            [
                2, 192, 6, 110, 16, 167, 42, 189, 43, 51, 195, 178, 20, 203, 62, 129, 188, 177, 182, 227,
                9, 97, 205, 35, 194, 2, 177, 134, 115, 191, 37, 67,
            ],
            [
                8, 44, 156, 55, 10, 13, 36, 244, 65, 111, 188, 65, 74, 55, 104, 31, 120, 68, 45, 39, 216,
                99, 133, 153, 28, 23, 214, 252, 12, 75, 125, 113,
            ],
            [
                16, 56, 150, 5, 174, 104, 141, 79, 20, 219, 133, 49, 34, 196, 125, 102, 168, 3, 199, 43,
                65, 88, 156, 177, 191, 134, 135, 65, 178, 6, 185, 187,
            ],
            [
                42, 115, 246, 121, 50, 140, 62, 171, 114, 74, 163, 229, 189, 191, 80, 179, 144, 53, 215,
                114, 159, 19, 91, 151, 9, 137, 15, 133, 197, 220, 94, 118,
            ],
            [
                34, 118, 49, 10, 167, 243, 52, 58, 40, 66, 20, 19, 157, 157, 169, 89, 190, 42, 49, 178,
                199, 8, 165, 248, 25, 84, 178, 101, 229, 58, 48, 184,
            ],
            [
                23, 126, 20, 83, 196, 70, 225, 176, 125, 43, 66, 51, 66, 81, 71, 9, 92, 79, 202, 187, 35,
                61, 35, 11, 109, 70, 162, 20, 217, 91, 40, 132,
            ],
            [
                14, 143, 238, 47, 228, 157, 163, 15, 222, 235, 72, 196, 46, 187, 68, 204, 110, 231, 5, 95,
                97, 251, 202, 94, 49, 59, 138, 95, 202, 131, 76, 71,
            ],
            [
                46, 196, 198, 94, 99, 120, 171, 140, 115, 48, 133, 79, 74, 112, 119, 193, 255, 146, 96,
                228, 72, 133, 196, 184, 29, 209, 49, 173, 58, 134, 205, 150,
            ],
            [
                0, 113, 61, 65, 236, 166, 53, 241, 23, 212, 236, 188, 235, 95, 58, 102, 220, 65, 66, 235,
                112, 181, 103, 101, 188, 53, 143, 27, 236, 64, 187, 155,
            ],
            [
                20, 57, 11, 224, 186, 239, 36, 155, 212, 124, 101, 221, 172, 101, 194, 229, 46, 133, 19,
                192, 129, 193, 205, 114, 201, 128, 6, 9, 142, 154, 143, 190,
            ],
        ];

        let inputs: BN[] = [];
        let value: BN = new BN(1);

        for (let i = 0; i < TEST_CASES.length; i++) {
            inputs.push(value);
            const mod = await WasmHasher.loadModule();
            const hash = mod.create();
            const rsHash = hash.poseidonHash(inputs);
            assert.equal(TEST_CASES[i].toString(), Array.from(rsHash).toString());
        }

        inputs = [];
        value = new BN(2);
        for (let i = 0; i < TEST_CASES.length; i++) {
            inputs.push(value);
            const mod = await WasmHasher.loadModule();
            const hash = mod.create();
            const rsHash = hash.poseidonHash(inputs);
            assert.notEqual(TEST_CASES[i].toString(), Array.from(rsHash).toString());
        }
    })

});
