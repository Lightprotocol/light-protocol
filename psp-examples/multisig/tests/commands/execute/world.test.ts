import { expect, test } from "@oclif/test";

describe("execute world", () => {
  test
    .stdout()
    .command(["execute:world"])
    .it("runs execute world cmd", (ctx) => {
      expect(ctx.stdout).to.contain("execute world!");
    });
});
