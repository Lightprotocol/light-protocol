import test, { expect } from "@oclif/test";
import { initTestEnv } from "../../../src/utils/initTestEnv";

describe("With preloaded accounts", () => {
  before(async () => {
    await initTestEnv({});
  });
  test
    .stdout({ print: true })
    .command(["pool-type:list"])
    .it("List pool types", ({ stdout }) => {
      expect(stdout).to.contain("0, 0");
    });
});
