import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src";

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
        "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      );
    });
});
