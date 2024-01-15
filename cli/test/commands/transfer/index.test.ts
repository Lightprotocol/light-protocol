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
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      "--localTestRpc",
    ])
    .it("transfer 1.5 SOL to a compressed account address", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully transferred 1.5 SOL ✔");
    });

  test
    .stdout()
    .command([
      "transfer",
      "5",
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      "--token=usdc",
      "--localTestRpc",
    ])
    .it("transfer 5 USDC to a compressed account address", async (ctx) => {
      expect(ctx.stdout).to.contain("Successfully transferred 5 USDC ✔");
    });

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "5",
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmicFAIL",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail transfer to an invalid compressed recipient address");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "550000000",
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmicFAIL",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail transfer of unsufficient SOL amount");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "550000",
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmicFAIL",
      "--token=usdc",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail transfer of unsufficient SPL amount");

  test
    .stdout()
    .stderr()
    .command([
      "transfer",
      "5",
      "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      "--token=LFG",
      "--localTestRpc",
    ])
    .exit(2)
    .it("Should fail transfer of an unregistered SPL token");
});
