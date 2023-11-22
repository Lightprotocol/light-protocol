import { it } from "mocha";
import { Prover } from "../src";
import { IDL } from "./circuits/idl";
import { Poseidon } from "@lightprotocol/account.rs";
import { isBoolean, blake } from "@lightprotocol/accountwasm";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
chai.use(chaiAsPromised);

describe("Prover Functionality Tests", () => {
  it("Valid proof test", async () => {
    const poseidon = await Poseidon.getInstance();
    const hash = poseidon.hashString(["123"]);
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

  it.only("accountwasm", async () => {

    await initWasm();
    
    const x = 1;
    console.log("isBoolean", isBoolean(x));

    const blakeX = blake(x);
    console.log("blake: ", blakeX);
  });

  it("Testing invalid proof", async () => {
    const poseidon = await Poseidon.getInstance();
    const hash = poseidon.hashString(["123"]);

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
