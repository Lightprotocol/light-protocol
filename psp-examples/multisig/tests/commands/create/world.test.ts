import { expect, test } from "@oclif/test";

describe("create world", () => {
  test
    .stdout()
    .command(["create:world"])
    .it("runs create world cmd", (ctx) => {
      expect(ctx.stdout).to.contain("create multisig world!");
    });
});
