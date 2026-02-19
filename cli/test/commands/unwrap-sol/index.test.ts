import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("unwrap-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 200;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`full wrap-check-unwrap-check cycle for ${amount} SOL to ${to}`, async () => {
    // Get initial balance first
    const { stdout: initialBalanceStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    let initialBalance = 0;
    if (initialBalanceStdout.includes("No accounts found")) {
      initialBalance = 0;
    } else {
      const balanceMatch = initialBalanceStdout.match(
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
    const { stdout: afterWrapStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    const balanceMatchAfterWrap = afterWrapStdout.match(
      /Wrapped SOL balance:\s+(\d+)/,
    );
    expect(balanceMatchAfterWrap).to.not.be.null;

    let balanceAfterWrap = 0;
    if (balanceMatchAfterWrap && balanceMatchAfterWrap[1]) {
      balanceAfterWrap = parseInt(balanceMatchAfterWrap[1], 10);
      console.log(`Balance after wrapping: ${balanceAfterWrap}`);

      // Verify the balance increased by the wrapped amount
      expect(balanceAfterWrap).to.equal(initialBalance + amount);
    } else {
      throw new Error("Could not extract balance from output");
    }

    // Unwrap SOL
    const { stdout: unwrapStdout } = await runCommand([
      "unwrap-sol",
      `--amount=${amount}`,
      `--to=${to}`,
    ]);
    expect(unwrapStdout).to.contain("unwrap-sol successful");

    // Check balance after unwrapping
    const { stdout: finalBalanceStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    // Extract the final balance
    if (finalBalanceStdout.includes("No accounts found")) {
      // If there were no accounts before wrapping, there should be none after unwrapping
      expect(initialBalance).to.equal(0);
    } else {
      const balanceMatch = finalBalanceStdout.match(
        /Wrapped SOL balance:\s+(\d+)/,
      );
      if (balanceMatch && balanceMatch[1]) {
        const finalBalance = parseInt(balanceMatch[1], 10);
        console.log(
          `Final balance: ${finalBalance}, Expected: ${balanceAfterWrap - amount}`,
        );

        // Verify the balance decreased by the unwrapped amount
        expect(finalBalance).to.equal(balanceAfterWrap - amount);
      } else {
        // If we can't extract the balance but initial balance was equal to amount,
        // we should get "No accounts found"
        if (balanceAfterWrap === amount) {
          expect(finalBalanceStdout).to.contain("No accounts found");
        } else {
          throw new Error("Could not extract balance from output");
        }
      }
    }
  });
});
