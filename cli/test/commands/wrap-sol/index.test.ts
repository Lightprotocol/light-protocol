import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("wrap-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 1000_000;
  let initialBalance = 0;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`wrap-sol ${amount} lamports to ${to} and verify balance increase`, async () => {
    // Get initial balance first
    const { stdout: initialStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    let initialBalance = 0;
    if (initialStdout.includes("No accounts found")) {
      initialBalance = 0;
    } else {
      // Extract the balance number
      const balanceMatch = initialStdout.match(
        /Wrapped SOL balance:\s+(\d+)/,
      );
      if (balanceMatch && balanceMatch[1]) {
        initialBalance = parseInt(balanceMatch[1], 10);
      }
    }
    console.log(`Initial balance captured: ${initialBalance}`);

    // Wrap SOL
    const { stdout: wrapStdout } = await runCommand([
      "wrap-sol",
      `--amount=${amount}`,
      `--to=${to}`,
    ]);
    expect(wrapStdout).to.contain("wrap-sol successful");

    // Check balance after wrapping
    const { stdout: finalStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    // Extract the new balance
    const balanceMatch = finalStdout.match(/Wrapped SOL balance:\s+(\d+)/);
    expect(balanceMatch).to.not.be.null;

    if (balanceMatch && balanceMatch[1]) {
      const newBalance = parseInt(balanceMatch[1], 10);
      console.log(
        `New balance: ${newBalance}, Initial balance: ${initialBalance}, Expected increase: ${amount}`,
      );

      // Verify the balance increased by the wrapped amount
      expect(newBalance).to.equal(initialBalance + amount);
    } else {
      throw new Error("Could not extract balance from output");
    }
  });
});
