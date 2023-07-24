import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src";

describe("unshield SOL & SPL separately with the main command", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });

  test
    .stdout({ print: true })
    .command([
      "unshield",
      "--amount-sol=0.2",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .it("Unshielding 0.2 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully unshielded 0.2 SOL ✔");
    });

  test
    .stdout({ print: true })
    .command([
      "unshield",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .it("Unshielding 0.5 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully unshielded 0.5 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "unshield",
      "--amount-sol=3000000",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of insufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "unshield",
      "--amount-spl=5500000",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "unshield",
      "--amount-spl=0.5",
      "--token=LFG",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of unregistered SPL token");
});

describe("unshield SOL & SPL at the same time with the main command", () => {
  test
    .stdout()
    .command([
      "unshield",
      "--amount-sol=0.2",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .it(
      "Unshielding 0.2 SOL and 0.5 SPL:USDC at the same time with the main cli",
      async (ctx) => {
        expect(ctx.stdout).to.contain(
          "Successfully unshielded 0.2 SOL & 0.5 USDC ✔"
        );
      }
    );

  test
    .stdout()
    .stderr()
    .command([
      "unshield",
      "--amount-sol=2200000",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of insufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "unshield",
      "--amount-sol=0.2",
      "--amount-spl=35000000",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "unshield",
      "--amount-sol=0.2",
      "--amount-spl=0.5",
      "--token=LFG",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail unshield of unregistered SPL token");
});
