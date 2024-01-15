import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("decompress:spl", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout()
    .command([
      "decompress:spl",
      "0.5",
      "USDC",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .it("Decompressing 0.5 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully decompressed 0.5 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "decompress",
      "550000",
      "USDC",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "decompress",
      "0.5",
      "LFG",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of unregistered SPL token");
});
