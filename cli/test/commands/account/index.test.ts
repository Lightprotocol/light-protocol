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
        "HPLohgqzaUuyYVJtSgDk4iVJdXRX2FXHkYPcdYH23whnJUdxty2ZrjjGVdKaQAqgyCmg9ecYtKYQfppsgQaA84q",
      );
    });
});
