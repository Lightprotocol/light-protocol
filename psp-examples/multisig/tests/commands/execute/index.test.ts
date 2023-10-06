import { expect, test } from "@oclif/test";

describe("execute", () => {
  test
    .stdout()
    .command(["execute", "friend", "--from=oclif"])
    .it("runs execute cmd", (ctx) => {
      expect(ctx.stdout).to.contain("execute friend from oclif!");
    });
});
