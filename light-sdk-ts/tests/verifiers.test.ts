import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const should = chai.should();
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";

import {
  functionalCircuitTest,
  VerifierZero,
  VerifierTwo,
  VerifierOne,
  VerifierError,
  VerifierErrorCode,
  TransactionErrorCode,
} from "../src";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  { verifier: new VerifierZero(), isApp: false },
  { verifier: new VerifierOne(), isApp: false },
  { verifier: new VerifierTwo(), isApp: true },
];

describe("Verifier tests", () => {
  let poseidon;
  before(async () => {
    poseidon = await buildPoseidonOpt();
  });

  it("Test functional circuit", async () => {
    for (var verifier in verifiers) {
      await functionalCircuitTest(
        verifiers[verifier].verifier,
        verifiers[verifier].isApp,
      );
    }
  });

  it("Public inputs: INVALID_INPUTS_NUMBER", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        verifiers[verifier].verifier.parsePublicInputsFromArray([[]]);
      })
        .throw(VerifierError)
        .includes({
          code: VerifierErrorCode.INVALID_INPUTS_NUMBER,
          functionName: "parsePublicInputsFromArray",
        });
    }
  });

  it("PUBLIC_INPUTS_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        // @ts-ignore: for test
        verifiers[verifier].verifier.parsePublicInputsFromArray();
      })
        .throw(VerifierError)
        .includes({
          code: VerifierErrorCode.PUBLIC_INPUTS_UNDEFINED,
          functionName: "parsePublicInputsFromArray",
        });
    }
  });
});
