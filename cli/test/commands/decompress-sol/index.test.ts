import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("decompress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 200;
  
  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`decompresses ${amount} SOL to ${to}`, async () => {
    const compressResult = await runCommand([
      "compress-sol",
      `--amount=${amount}`,
      `--to=${to}`
    ]);
    expect(compressResult.error).toBeUndefined();
    expect(compressResult.stdout).toContain("compress-sol successful");

    const decompressResult = await runCommand([
      "decompress-sol",
      `--amount=${amount}`,
      `--to=${to}`
    ]);
    expect(decompressResult.error).toBeUndefined();
    expect(decompressResult.stdout).toContain("decompress-sol successful");
  });
});
