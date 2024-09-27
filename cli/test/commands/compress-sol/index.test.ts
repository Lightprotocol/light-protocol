import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("compress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 1000_000;

  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  it(`compresses ${amount} lamports to ${to}`, async () => {
    const result = await runCommand([
      "compress-sol",
      `--amount=${amount}`,
      `--to=${to}`
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("compress-sol successful");
  });
});
