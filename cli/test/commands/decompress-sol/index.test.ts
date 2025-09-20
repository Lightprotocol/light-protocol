import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("decompress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 200;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`full compress-check-decompress-check cycle for ${amount} SOL to ${to}`, async () => {
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
        /Compressed SOL balance:\s+(\d+)/,
      );
      if (balanceMatch && balanceMatch[1]) {
        initialBalance = parseInt(balanceMatch[1], 10);
      }
    }
    console.log(`Initial balance captured: ${initialBalance}`);

    // Compress SOL
    const { stdout: compressStdout } = await runCommand([
      "compress-sol",
      `--amount=${amount}`,
      `--to=${to}`,
    ]);
    expect(compressStdout).to.contain("compress-sol successful");

    // Check balance after compression
    const { stdout: afterCompressStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    const balanceMatchAfterCompress = afterCompressStdout.match(
      /Compressed SOL balance:\s+(\d+)/,
    );
    expect(balanceMatchAfterCompress).to.not.be.null;

    let balanceAfterCompression = 0;
    if (balanceMatchAfterCompress && balanceMatchAfterCompress[1]) {
      balanceAfterCompression = parseInt(balanceMatchAfterCompress[1], 10);
      console.log(`Balance after compression: ${balanceAfterCompression}`);

      // Verify the balance increased by the compressed amount
      expect(balanceAfterCompression).to.equal(initialBalance + amount);
    } else {
      throw new Error("Could not extract balance from output");
    }

    // Decompress SOL
    const { stdout: decompressStdout } = await runCommand([
      "decompress-sol",
      `--amount=${amount}`,
      `--to=${to}`,
    ]);
    expect(decompressStdout).to.contain("decompress-sol successful");

    // Check balance after decompression
    const { stdout: finalBalanceStdout } = await runCommand([
      "balance",
      `--owner=${to}`,
    ]);

    // Extract the final balance
    if (finalBalanceStdout.includes("No accounts found")) {
      // If there were no accounts before compression, there should be none after decompression
      expect(initialBalance).to.equal(0);
    } else {
      const balanceMatch = finalBalanceStdout.match(
        /Compressed SOL balance:\s+(\d+)/,
      );
      if (balanceMatch && balanceMatch[1]) {
        const finalBalance = parseInt(balanceMatch[1], 10);
        console.log(
          `Final balance: ${finalBalance}, Expected: ${balanceAfterCompression - amount}`,
        );

        // Verify the balance decreased by the decompressed amount
        expect(finalBalance).to.equal(balanceAfterCompression - amount);
      } else {
        // If we can't extract the balance but initial balance was equal to amount,
        // we should get "No accounts found"
        if (balanceAfterCompression === amount) {
          expect(finalBalanceStdout).to.contain("No accounts found");
        } else {
          throw new Error("Could not extract balance from output");
        }
      }
    }
  });
});
