import test, { expect } from "@oclif/test";
import { initTestEnv } from "../../../src/utils/initTestEnv";

describe("With preloaded accounts", () => {
  before(async () => {
    await initTestEnv({});
  });
  test
    .stdout()
    .command(["asset-pool:list"])
    .it("List asset pols", ({ stdout }) => {
      expect(stdout).to.contain("2mobV36eNyF");
    });
});
