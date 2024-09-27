import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import * as fs from "fs";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { DEFAULT_CONFIG } from "../../../src/utils/constants";

describe("config", () => {
  beforeAll(async () => {
    await initTestEnvIfNeeded();
  });

  it("runs solana rpc url update cmd", async () => {
    const result = await runCommand(["config", "--solanaRpcUrl=http://127.0.0.1:8899"]);
    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("Configuration values updated successfully");
  });
});

let filePath = process.env.INIT_CWD + "/config.json";
describe("config with env variable", () => {
  beforeAll(async () => {
    await initTestEnvIfNeeded();
    console.log("export LIGHT_PROTOCOL_CONFIG=" + filePath);
    process.env.LIGHT_PROTOCOL_CONFIG = filePath;
    let data = {
      ...DEFAULT_CONFIG,
    };

    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  });

  it("runs solana rpc url update cmd with custom config path", async () => {
    const result = await runCommand(["config", "--solanaRpcUrl=http://127.0.0.1:8899"]);
    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain(`reading config from custom path ${filePath}`);
  });
});

describe("test-validator stop", () => {
  beforeAll(async () => {
    await initTestEnvIfNeeded();
  });

  it("runs test-validator stop cmd", async () => {
    const result = await runCommand(["test-validator", "--stop"]);
    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("Test validator stopped successfully");
  });
});
