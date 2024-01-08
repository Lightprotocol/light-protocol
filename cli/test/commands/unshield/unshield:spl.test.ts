import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("unshield:spl", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout()
    .command([
      "unshield:spl",
      "0.5",
      "USDC",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .it("Unshielding 0.5 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully unshielded 0.5 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "unshield",
      "550000",
      "USDC",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail unshield of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "unshield",
      "0.5",
      "LFG",
      "E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail unshield of unregistered SPL token");
});
