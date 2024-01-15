import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("compress:sol sub-cli", () => {
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
    .command(["compress:sol", "2.3", "--localTestRpc"])
    .it("Shielding 2.3 SOL", async (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully compressed 2.3 SOL ✔");
    });

  test
    .stdout()
    .command(["compress:sol", "123456789", "-d", "--localTestRpc"])
    .it("Shielding 123456789 LAMPORTS", async (ctx) => {
      expect(ctx.stdout).to.contain(
        "Successfully compressed 0.123456789 SOL ✔",
      );
    });

  test
    .stdout()
    .stderr()
    .command([
      "compress:sol",
      "2222222222222222222222222222222222222222",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail compress of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "compress:sol",
      "0.5",
      "--localTestRpc",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL",
    ])
    .exit(2)
    .it("Should fail compress SOL to an invalid compressed recipient address");
});
