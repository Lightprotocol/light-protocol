import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("compress:spl sub-cli", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
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
    .stdout({ print: true })
    .command(["compress:spl", "10", "USDC", "--localTestRpc"])
    .it("compressing 1 USDC", (ctx) => {
      expect(ctx.stdout).to.contain("Successfully compressed 10 USDC ✔");
    });

  test
    .stdout({ print: true })
    .command(["compress:spl", "123", "USDC", "-d", "--localTestRpc"])
    .it("compressing 1.23 USDC taking absolute input with the subcli", (ctx) => {
      expect(ctx.stdout).to.contain("Successfully compressed 1.23 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "compress:spl",
      "10000000000000000000000000000000000000000",
      "USDC",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress:spl",
      "10",
      "USDC",
      "--recipient=HPLohgqzaUuyYVJtSgDk4iVJdXRX2FXHkYPcdYH23whnJUdxty2ZrjjGVdKaQAqgyCmg9ecYtKYQfppsgQaA84qFAIL",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress SPL to an invalid compressed recipient address");
});
