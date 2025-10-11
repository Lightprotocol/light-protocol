import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("decompress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 200;
  let initialBalance = 0;
  let balanceAfterCompression = 0;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  test
    // Get initial balance first
    .stdout({ print: true })
    .command(["balance", `--owner=${to}`])
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
    // Compress SOL
    .stdout({ print: true })
    .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
    .do((ctx) => {
      expect(ctx.stdout).to.contain("compress-sol successful");
    })
    // Check balance after compression
    .stdout({ print: true })
    .command(["balance", `--owner=${to}`])
    .do((ctx) => {
      // Extract the new balance after compression
      const balanceMatch = ctx.stdout.match(/Compressed SOL balance:\s+(\d+)/);
      expect(balanceMatch).to.not.be.null;

      if (balanceMatch && balanceMatch[1]) {
        balanceAfterCompression = parseInt(balanceMatch[1], 10);
        console.log(`Balance after compression: ${balanceAfterCompression}`);

        // Verify the balance increased by the compressed amount
        expect(balanceAfterCompression).to.equal(initialBalance + amount);
      } else {
        throw new Error("Could not extract balance from output");
      }
    })
    // Decompress SOL
    .stdout({ print: true })
    .command(["decompress-sol", `--amount=${amount}`, `--to=${to}`])
    .do((ctx) => {
      expect(ctx.stdout).to.contain("decompress-sol successful");
    })
    // Check balance after decompression
    .stdout({ print: true })
    .command(["balance", `--owner=${to}`])
    .it(
      `full compress-check-decompress-check cycle for ${amount} SOL to ${to}`,
      (ctx) => {
        // Extract the final balance
        if (ctx.stdout.includes("No accounts found")) {
          // If there were no accounts before compression, there should be none after decompression
          expect(initialBalance).to.equal(0);
        } else {
          const balanceMatch = ctx.stdout.match(
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
              expect(ctx.stdout).to.contain("No accounts found");
            } else {
              throw new Error("Could not extract balance from output");
            }
          }
        }
      },
    );
});
