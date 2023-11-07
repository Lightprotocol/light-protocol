import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("shield:spl sub-cli", () => {
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
      }
    );

  test
    .stdout({ print: true })
    .command(["shield:spl", "10", "USDC", "--localTestRelayer"])
    .it("shielding 1 USDC", (ctx) => {
      expect(ctx.stdout).to.contain("Successfully shielded 10 USDC ✔");
    });

  test
    .stdout({ print: true })
    .command(["shield:spl", "123", "USDC", "-d", "--localTestRelayer"])
    .it("shielding 1.23 USDC taking absolute input with the subcli", (ctx) => {
      expect(ctx.stdout).to.contain("Successfully shielded 1.23 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "shield:spl",
      "10000000000000000000000000000000000000000",
      "USDC",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail shield of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "shield:spl",
      "10",
      "USDC",
      "--recipient=DVTtJhghZU1hBEbCci4RDpRP1K1eEHZXyYognZ4BNiCBaM8WenG3o6v8CNcKTRD7fVUsSTtae8hU5To1ogrGQDwFAIL",
      "--localTestRelayer",
    ])
    .exit(2)
    .it("Should fail shield SPL to an invalid shielded recipient address");
});
