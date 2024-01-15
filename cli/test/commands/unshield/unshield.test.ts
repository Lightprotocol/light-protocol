import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src";

describe("decompress SOL & SPL separately with the main command", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });

  test
    .stdout({ print: true })
    .command([
      "decompress",
      "--amount-sol=0.2",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .it("Decompressing 0.2 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully decompressed 0.2 SOL ✔");
    });

  test
    .stdout({ print: true })
    .command([
      "decompress",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
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
      "--amount-sol=3000000",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of insufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "decompress",
      "--amount-spl=5500000",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "decompress",
      "--amount-spl=0.5",
      "--token=LFG",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of unregistered SPL token");
});

describe("decompress SOL & SPL at the same time with the main command", () => {
  test
    .stdout({ print: true })
    .command([
      "decompress",
      "--amount-sol=0.2",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .it(
      "Decompressing 0.2 SOL and 0.5 SPL:USDC at the same time with the main cli",
      async (ctx) => {
        expect(ctx.stdout).to.contain(
          "Successfully decompressed 0.2 SOL & 0.5 USDC ✔",
        );
      },
    );

  test
    .stdout()
    .stderr()
    .command([
      "decompress",
      "--amount-sol=2200000",
      "--amount-spl=0.5",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of insufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "decompress",
      "--amount-sol=0.2",
      "--amount-spl=35000000",
      "--token=USDC",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of insufficient SPL token amount");

  test
    .stdout()
    .command([
      "decompress",
      "--amount-sol=0.2",
      "--amount-spl=0.5",
      "--token=LFG",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail decompress of unregistered SPL token");
});
