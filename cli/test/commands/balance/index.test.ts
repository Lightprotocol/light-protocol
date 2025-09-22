import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("balance", () => {
  const keypair = defaultSolanaWalletKeypair();
  const owner = keypair.publicKey.toBase58();
  const amount = 200;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`get compressed SOL balance for ${owner}`, async () => {
    // Get initial balance first
    const { stdout: initialStdout } = await runCommand([
      "balance",
      `--owner=${owner}`,
    ]);

    let initialBalance = 0;
    if (initialStdout.includes("No accounts found")) {
      initialBalance = 0;
    } else {
      const balanceMatch = initialStdout.match(
        /Compressed SOL balance:\s+(\d+)/,
      );
      if (balanceMatch && balanceMatch[1]) {
        initialBalance = parseInt(balanceMatch[1], 10);
      }
    }
    console.log(`Initial balance captured: ${initialBalance}`);

    // Compress SOL to create a balance to check
    const { stdout: compressStdout } = await runCommand([
      "compress-sol",
      `--amount=${amount}`,
      `--to=${owner}`,
    ]);
    expect(compressStdout).to.contain("compress-sol successful");

    // Test the balance command
    const { stdout: finalStdout } = await runCommand([
      "balance",
      `--owner=${owner}`,
    ]);

    // Extract the balance
    const balanceMatch = finalStdout.match(/Compressed SOL balance:\s+(\d+)/);
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
