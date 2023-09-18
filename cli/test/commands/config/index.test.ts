import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { DEFAULT_CONFIG } from "../../../src/psp-utils";
import * as fs from "fs";

describe("config", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
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

let filePath = process.cwd() + "/config.json";
describe("config with env variable", () => {
  before(async () => {
    await initTestEnvIfNeeded();
    console.log("export LIGHT_PROTOCOL_CONFIG=" + filePath);
    process.env.LIGHT_PROTOCOL_CONFIG = filePath;
    let data = {
      ...DEFAULT_CONFIG,
      // TODO: remove this default secret key which we need for tests right now
      secretKey:
        "LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8",
    };

    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  });

  test
    .only()
    .stdout({ print: true })
    .command(["config", "--rpcUrl=http://127.0.0.1:8899"])
    .it("runs rpc url update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        `reading config from custom path ${filePath}`
      );
    });
});
