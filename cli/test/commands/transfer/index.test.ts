import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("transfer", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout()
    .command([
      "transfer",
      "1.5",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e",
    ])
    .it("transfer 1.5 SOL to a shielded account address", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully transferred 1.5 SOL ✔");
    });

  test
    .stdout()
    .command([
      "transfer",
      "5",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e",
      "--token=usdc",
    ])
    .it("transfer 5 USDC to a shielded account address", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully transferred 5 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "5",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL",
    ])
    .exit(2)
    .it("Should fail transfer to an invalid shielded recipient address");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "550000000",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL",
    ])
    .exit(2)
    .it("Should fail transfer of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "550000",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL",
      "--token=usdc",
    ])
    .exit(2)
    .it("Should fail transfer of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "5",
      "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e",
      "--token=LFG",
    ])
    .exit(2)
    .it("Should fail transfer of an unregistered SPL token");
});
