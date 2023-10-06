import { expect, test } from "@oclif/test";

describe("approve world", () => {
  test
    .stdout()
    .command(["approve:world"])
    .it("runs approve world cmd", (ctx) => {
      expect(ctx.stdout).to.contain("approve world!");
    });
});
