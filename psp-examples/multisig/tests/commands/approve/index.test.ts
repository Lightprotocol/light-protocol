import { expect, test } from "@oclif/test";

describe("approve", () => {
  test
    .stdout()
    .command(["approve", "friend", "--from=oclif"])
    .it("runs approve cmd", (ctx) => {
      expect(ctx.stdout).to.contain("approve friend from oclif!");
    });
});
