import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("shield:sol sub-cli", () => {
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
    .command(["shield:sol", "2.3", "--localTestRelayer"])
    .it("Shielding 2.3 SOL", async (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded 2.3 SOL ✔");
    });

  test
    .stdout()
    .command(["shield:sol", "123456789", "-d", "--localTestRelayer"])
    .it("Shielding 123456789 LAMPORTS", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully shielded 0.123456789 SOL ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "shield:sol",
      "2222222222222222222222222222222222222222",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail shield of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "shield:sol",
      "0.5",
      "--localTestRelayer",
      "--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL",
    ])
    .exit(2)
    .it("Should fail shield SOL to an invalid shielded recipient address");
});
