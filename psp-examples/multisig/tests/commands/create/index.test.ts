import { expect, test } from "@oclif/test";

describe("create", () => {
  test
    .stdout()
    .command(["create", "friend", "--from=oclif"])
    .it("runs create cmd", (ctx) => {
      expect(ctx.stdout).to.contain("create multisig friend from oclif!");
    });
});
