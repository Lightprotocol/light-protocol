import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("balance", () => {
  const keypair = defaultSolanaWalletKeypair();
  const owner = keypair.publicKey.toBase58();
  const amount = 200;
  let initialBalance = 0;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  test
    // Get initial balance first
    .stdout({ print: true })
    .command(["balance", `--owner=${owner}`])
    .do((ctx) => {
      // Capture initial balance or set to 0 if no accounts found
      if (ctx.stdout.includes("No accounts found")) {
        initialBalance = 0;
      } else {
        // Extract the balance number
        const balanceMatch = ctx.stdout.match(
          /Compressed SOL balance:\s+(\d+)/,
        );
        if (balanceMatch && balanceMatch[1]) {
          initialBalance = parseInt(balanceMatch[1], 10);
        }
      }
      console.log(`Initial balance captured: ${initialBalance}`);
    })
    // Compress SOL to create a balance to check
    .stdout({ print: true })
    .command(["compress-sol", `--amount=${amount}`, `--to=${owner}`])
    .do((ctx) => {
      expect(ctx.stdout).to.contain("compress-sol successful");
    })
    // Test the balance command
    .stdout({ print: true })
    .command(["balance", `--owner=${owner}`])
    .it(`get compressed SOL balance for ${owner}`, (ctx) => {
      // Extract the balance
      const balanceMatch = ctx.stdout.match(/Compressed SOL balance:\s+(\d+)/);
      expect(balanceMatch).to.not.be.null;

      if (balanceMatch && balanceMatch[1]) {
        const currentBalance = parseInt(balanceMatch[1], 10);
        console.log(
          `Current balance: ${currentBalance}, Initial balance: ${initialBalance}, Expected increase: ${amount}`,
        );

        // Verify the balance increased by the compressed amount
        expect(currentBalance).to.equal(initialBalance + amount);
      } else {
        throw new Error("Could not extract balance from output");
      }
    });
});
