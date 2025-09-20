import { runCommand } from "@oclif/test";
import { expect } from "chai";
import * as fs from "fs";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { DEFAULT_CONFIG } from "../../../src/utils/constants";

describe("config", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });

  it("runs solana rpc url update cmd", async () => {
    const { stdout } = await runCommand([
      "config",
      "--solanaRpcUrl=http://127.0.0.1:8899",
    ]);
    expect(stdout).to.contain("Configuration values updated successfully");
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

  it("runs solana rpc url update cmd", async () => {
    const { stdout } = await runCommand([
      "config",
      "--solanaRpcUrl=http://127.0.0.1:8899",
    ]);
    expect(stdout).to.contain(`reading config from custom path ${filePath}`);
  });
});
