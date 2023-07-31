import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("account", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout({ print: true })
    .command(["account"])
    .exit(0)
    .it("runs account cmd", ({ stdout }) => {
      expect(stdout).to.contain(
        "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e"
      );
    });
});
