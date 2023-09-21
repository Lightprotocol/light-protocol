const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { it } from "mocha";

import {
  functionalCircuitTest,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_TWO,
} from "../src";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  { verifierIdl: IDL_VERIFIER_PROGRAM_ZERO, isApp: false },
  { verifierIdl: IDL_VERIFIER_PROGRAM_ONE, isApp: false },
  { verifierIdl: IDL_VERIFIER_PROGRAM_TWO, isApp: true },
];

describe("Verifier tests", () => {
  it("Test functional circuit", async () => {
    for (let verifier in verifiers) {
      await functionalCircuitTest(
        verifiers[verifier].isApp,
        verifiers[verifier].verifierIdl,
      );
    }
  });
});
