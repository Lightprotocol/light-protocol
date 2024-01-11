const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { it } from "mocha";

import {
  functionalCircuitTest,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
} from "../src";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  { verifierIdl: IDL_LIGHT_PSP2IN2OUT, isApp: false },
  { verifierIdl: IDL_LIGHT_PSP10IN2OUT, isApp: false },
  { verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE, isApp: true },
];

describe("Verifier tests", () => {
  it("Test functional circuit", async () => {
    for (const verifier in verifiers) {
      console.log("verifier");
      await functionalCircuitTest(
        verifiers[verifier].isApp,
        verifiers[verifier].verifierIdl,
      );
    }
  });
});
