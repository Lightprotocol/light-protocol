import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import {
  createTestMint,
  requestAirdrop,
  testMintTo,
} from "../../helpers/helpers";

describe("compress-spl", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;

  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);

    await createTestMint(mintKeypair);

    await testMintTo(
      payerKeypair,
      mintKeypair.publicKey,
      payerKeypair.publicKey,
      mintAuthority,
      mintAmount,
    );
  });

  it(`compresses ${mintAmount - 2} tokens to ${payerKeypair.publicKey.toBase58()} from ${payerKeypair.publicKey.toBase58()}`, async () => {
    const decompressResult = await runCommand([
      "decompress-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 1}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);

    expect(decompressResult.error).toBeUndefined();
    expect(decompressResult.stdout).toContain("decompress-spl successful");

    const compressResult = await runCommand([
      "compress-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 2}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);

    expect(compressResult.error).toBeUndefined();
    expect(compressResult.stdout).toContain("compress-spl successful");
  });
});
