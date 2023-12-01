import { it } from "mocha";
import { Prover } from "../src";
import { IDL } from "./circuits/idl";
import {WasmHash } from "@lightprotocol/account.rs";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
chai.use(chaiAsPromised);

describe("Prover Functionality Tests", () => {
  it("Valid proof test", async () => {
    const hasher = (await WasmHash.loadModule()).create();
    const hash = hasher.poseidonHashString(["123"]);
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
    const hasher = (await WasmHash.loadModule()).create();
    const hash = hasher.poseidonHashString(["123"]);

    const circuitsPath: string = "./tests/circuits/build-circuits";
    const proofInputs: any = {
      x: 1,
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath);

    await prover.addProofInputs(proofInputs);

    console.time("Proof generation + Parsing");
    await chai.assert.isRejected(prover.fullProveAndParse(), Error);
    console.timeEnd("Proof generation + Parsing");
  });

  after(async () => {
    // @ts-ignore
    if (globalThis.curve_bn128 !== null) {
      // @ts-ignore
      globalThis.curve_bn128.terminate();
    }
  });
});
