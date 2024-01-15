import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("compress SOL & SPL separately with the main command", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });

  test
    .stdout()
    .command(["airdrop", "50", "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k"])
    .it(
      "airdrop 50 SOL to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx: any) => {
        expect(ctx.stdout).to.contain("Airdrop Successful ✔");
      },
    );

  test
    .stdout()
    .command([
      "airdrop",
      "1000",
      "--token=USDC",
      "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
    ])
    .it(
      "airdrop 1000 USDC to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx: any) => {
        expect(ctx.stdout).to.contain("Airdrop Successful ✔");
      },
    );

  test
    .stdout()
    .command(["compress", "--amount-sol=7", "--localTestRpc"])
    .it("Shielding 7 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully compressed 7 SOL ✔");
    });

  test
    .stdout({ print: true })
    .command(["compress", "--amount-spl=9", "--token=USDC", "--localTestRpc"])
    .it("Shielding 9 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully compressed 9 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=22222222222222222222222222222222",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-spl=5555555555555555555555555555555",
      "--token=USDC",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=0.2",
      "--recipient=TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress to invalid compressed recipient address");

  test
    .stdout()
    .stderr()
    .command(["compress", "--amount-spl=3", "--token=LFG", "--localTestRpc"])
    .exit(2)
    .it("Should fail compress of unregistered SPL token");
});

describe("compress SOL & SPL at the same time with the main command", () => {
  test
    .stdout()
    .command([
      "compress",
      "--amount-sol=5",
      "--amount-spl=1",
      "--token=USDC",
      "--localTestRpc",
    ])
    .it(
      "Shielding 5 SOL & 1 SPL:USDC at the same time with the main cli",
      async (ctx) => {
        expect(ctx.stdout).to.contain(
          "Successfully compressed 5 SOL & 1 USDC ✔",
        );
      },
    );

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=222222222222222222222222222222222",
      "--amount-spl=3",
      "--token=USDC",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=0.2",
      "--amount-spl=33333333333333333333333333333333",
      "--token=USDC",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=0.2",
      "--amount-spl=33",
      "--token=USDC",
      "--recipient=TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress to invalid compressed recipient address");

  test
    .stdout()
    .stderr()
    .command([
      "compress",
      "--amount-sol=0.2",
      "--amount-spl=3",
      "--token=LFG",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unregistered SPL token");
});
