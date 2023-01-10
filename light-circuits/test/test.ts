

import { functionalCircuitTest } from "../../light-sdk-ts/src";

describe("verifier_program", () => {
// test functional circuit
it("Test functional circuit", async () => {
    await functionalCircuitTest();
  })
})