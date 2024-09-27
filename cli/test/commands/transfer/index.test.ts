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

describe("transfer", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const payerKeypairPath = process.env.HOME + "/.config/solana/id.json";

  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;
  const mintDestination = Keypair.generate().publicKey;

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

  it(`transfers ${mintAmount - 1} tokens to ${mintDestination.toBase58()} from ${mintKeypair.publicKey.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "transfer",
      `--amount=${mintAmount - 1}`,
      `--fee-payer=${payerKeypairPath}`,
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--to=${mintDestination.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("transfer successful");
  });
});
