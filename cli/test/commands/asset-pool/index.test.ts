import test, { expect } from "@oclif/test";
import { initTestEnv, killTestValidator } from "../../../src/utils/initTestEnv";

describe("Without preloaded accounts", () => {
  before(async () => {
    await initTestEnv({ skip_system_accounts: true });
  });
  // Other tests require a validator with system accounts. Kill the current one
  // which doesn't have them.
  after(async () => {
    await killTestValidator();
  });

  // TODO(vadorovsky): Teach `initTestEnv` to initialize only some accounts,
  // then we will be able to not initialize the Merkle Tree Authority this
  // way.
  test
    .stdout()
    .command(["merkle-tree-authority:initialize"])
    .it("Initialize Merkle Tree Authority", ({ stdout }) => {
      expect(stdout).to.contain(
        "Merkle Tree Authority initialized successfully"
      );
    });
  test
    .stdout()
    .command(["pool-type:register", "0"])
    .it("Register pool type", ({ stdout }) => {
      expect(stdout).to.contain("Pool type registered successfully");
    });

  test
    .stdout()
    .command(["asset-pool:register-sol", "0"])
    .it("Register SOL pool", ({ stdout }) => {
      expect(stdout).to.contain("SOL pool registered successfully");
    });
  test
    .stdout()
    .command([
      "asset-pool:register-spl",
      "0",
      "ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe",
    ])
    .it("Register SPL pool", ({ stdout }) => {
      expect(stdout).to.contain("SPL pool registered successfully");
    });
});

describe("With preloaded accounts", () => {
  before(async () => {
    await initTestEnv({});
  });
  test
    .stdout()
    .command(["asset-pool:list"])
    .it("List asset pols", ({ stdout }) => {
      expect(stdout).to.contain("2mobV36eNyF");
    });
});
