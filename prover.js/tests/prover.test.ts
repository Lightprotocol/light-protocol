import { Prover } from "../src";
import { IDL } from "./circuits/idl";
import { describe, it, expect, afterAll } from "vitest";
import { WasmFactory } from "@lightprotocol/account.rs";
import { assert } from "chai";
import { BN } from "@coral-xyz/anchor";

describe("Prover Functionality Tests", () => {
  it("Valid proof test no compression", async () => {
    const lightWasm = await WasmFactory.getInstance();
    const hash = lightWasm.poseidonHashString(["123"]);
    const circuitsPath: string = "./tests/circuits/build-circuits";
    const proofInputs: any = {
      x: "123",
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath, "poseidon");

    await prover.addProofInputs(proofInputs);

    console.time("Proof generation + Parsing");
    await prover.fullProveAndParse(false);
    console.timeEnd("Proof generation + Parsing");
  });

  it("Testing invalid proof", async () => {
    const hasher = await WasmFactory.getInstance();
    const hash = hasher.poseidonHashString(["123"]);

    const circuitsPath: string = "./tests/circuits/build-circuits";
    const proofInputs: any = {
      x: 1,
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath);

    await prover.addProofInputs(proofInputs);

    console.time("Proof generation + Parsing");
    await expect(prover.fullProveAndParse(false)).rejects.toThrow(Error);
    console.timeEnd("Proof generation + Parsing");
  });

  it("Valid proof test proof compression", async () => {
    const lightWasm = await WasmFactory.getInstance();
    const hash = lightWasm.poseidonHashString(["123"]);
    const circuitsPath: string = "./tests/circuits/build-circuits";
    const proofInputs: any = {
      x: "123",
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath, "poseidon");

    await prover.addProofInputs(proofInputs);

    console.time("Proof generation + Parsing");
    await prover.fullProveAndParse();
    console.timeEnd("Proof generation + Parsing");
  });

  it("Testing invalid proof", async () => {
    const hasher = await WasmFactory.getInstance();
    const hash = hasher.poseidonHashString(["123"]);

    const circuitsPath: string = "./tests/circuits/build-circuits";
    const proofInputs: any = {
      x: 1,
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath);

    await prover.addProofInputs(proofInputs);

    console.time("Proof generation + Parsing");
    await expect(prover.fullProveAndParse()).rejects.toThrow(Error);
    console.timeEnd("Proof generation + Parsing");
  });

  it("testing proof compression ", async () => {
    const proofA = [
      45, 206, 255, 166, 152, 55, 128, 138, 79, 217, 145, 164, 25, 74, 120, 234,
      234, 217, 68, 149, 162, 44, 133, 120, 184, 205, 12, 44, 175, 98, 168, 172,
      20, 24, 216, 15, 209, 175, 106, 75, 147, 236, 90, 101, 123, 219, 245, 151,
      209, 202, 218, 104, 148, 8, 32, 254, 243, 191, 218, 122, 42, 81, 193, 84,
    ];

    let proofACompressed = proofA.slice(0, 32);
    const proofAY = new BN(proofACompressed.slice(32), 32, "be");
    const proofAYIsPositive = Prover.yElementIsPositiveG1(proofAY);
    assert.isTrue(proofAYIsPositive);
    proofACompressed[0] = Prover.addBitmaskToByte(
      proofACompressed[0],
      proofAYIsPositive,
    );
    assert.equal(proofACompressed[0], proofA[0]);

    const proofC = [
      41, 139, 183, 208, 246, 198, 118, 127, 89, 160, 9, 27, 61, 26, 123, 180,
      221, 108, 17, 166, 47, 115, 82, 48, 132, 139, 253, 65, 152, 92, 209, 53,
      37, 25, 83, 61, 252, 42, 181, 243, 16, 21, 2, 199, 123, 96, 218, 151, 253,
      86, 69, 181, 202, 109, 64, 129, 124, 254, 192, 25, 177, 199, 26, 50,
    ];
    let proofCCompressed = proofC.slice(0, 32);
    const proofCY = new BN(proofC.slice(32, 64), 32, "be");
    const proofCYIsPositive = Prover.yElementIsPositiveG1(proofCY);
    assert.isNotTrue(proofCYIsPositive);
    proofCCompressed[0] = Prover.addBitmaskToByte(
      proofCCompressed[0],
      proofCYIsPositive,
    );
    assert.equal(proofCCompressed[0], 169);

    const proofB = [
      40, 57, 233, 205, 180, 46, 35, 111, 215, 5, 23, 93, 12, 71, 118, 225, 7,
      46, 247, 147, 47, 130, 106, 189, 184, 80, 146, 103, 141, 52, 242, 25, 0,
      203, 124, 176, 110, 34, 151, 212, 66, 180, 238, 151, 236, 189, 133, 209,
      17, 137, 205, 183, 168, 196, 92, 159, 75, 174, 81, 168, 18, 86, 176, 56,
      16, 26, 210, 20, 18, 81, 122, 142, 104, 62, 251, 169, 98, 141, 21, 253,
      50, 130, 182, 15, 33, 109, 228, 31, 79, 183, 88, 147, 174, 108, 4, 22, 14,
      129, 168, 6, 80, 246, 254, 100, 218, 131, 94, 49, 247, 211, 3, 245, 22,
      200, 177, 91, 60, 144, 147, 174, 90, 17, 19, 189, 62, 147, 152, 18,
    ];
    let proofBCompressed = proofB.slice(0, 64);
    const proofBY = [
      new BN(proofB.slice(64, 96), 32, "be"),
      new BN(proofB.slice(96, 128), 32, "be"),
    ];
    const proofBYIsPositive = Prover.yElementIsPositiveG2(
      proofBY[0],
      proofBY[1],
    );
    assert.isTrue(proofBYIsPositive);
    proofBCompressed[0] = Prover.addBitmaskToByte(
      proofBCompressed[0],
      proofBYIsPositive,
    );
    assert.equal(proofBCompressed[0], 40);
  });

  afterAll(async () => {
    // @ts-ignore
    if (globalThis.curve_bn128 !== null) {
      // @ts-ignore
      globalThis.curve_bn128.terminate();
    }
  });
});
