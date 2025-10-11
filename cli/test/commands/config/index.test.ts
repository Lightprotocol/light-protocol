import { expect, test } from "@oclif/test";
import * as fs from "fs";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { DEFAULT_CONFIG } from "../../../src/utils/constants";

describe("config", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout()
    .command(["config", "--solanaRpcUrl=http://127.0.0.1:8899"])
    .it("runs solana rpc url update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        "Configuration values updated successfully",
      );
    });
});

let filePath = process.env.INIT_CWD + "/config.json";
describe("config with env variable", () => {
  before(async () => {
    await initTestEnvIfNeeded();
    console.log("export LIGHT_PROTOCOL_CONFIG=" + filePath);
    process.env.LIGHT_PROTOCOL_CONFIG = filePath;
    let data = {
      ...DEFAULT_CONFIG,
    };

    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  });

  test
    .stdout({ print: true })
    .command(["config", "--solanaRpcUrl=http://127.0.0.1:8899"])
    .it("runs solana rpc url update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        `reading config from custom path ${filePath}`,
      );
    });
});

// Test the --stop flag
describe("test-validator stop", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });

  test
    .stdout()
    .command(["test-validator", "--stop"])
    .it("runs test-validator stop cmd", (ctx) => {
      expect(ctx.stdout).to.contain("Test validator stopped successfully");
    });
});
