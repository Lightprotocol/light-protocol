import { expect, test } from "@oclif/test";

describe("configuration", () => {
  test
    .stdout()
    .command(["config", "--relayerUrl=http://localhost:3331"])
    .it("runs relayer url update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        "Configuration values updated successfully"
      );
    });
  test
    .stdout()
    .command([
      "config",
      "--secretKey=LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8",
    ])
    .it("runs user update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        "Configuration values updated successfully"
      );
    });
  test
    .stdout()
    .command(["config", "--rpcUrl=http://127.0.0.1:8899"])
    .it("runs rpc url update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        "Configuration values updated successfully"
      );
    });
});
