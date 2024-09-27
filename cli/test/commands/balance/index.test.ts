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

describe("Get balance", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
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
      mintDestination,
      mintAuthority,
      mintAmount,
    );
  });

  it(`gets balance of ${mintAmount} tokens for ${mintDestination.toBase58()} from mint ${mintKeypair.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "balance",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--owner=${mintDestination.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("balance successful");
    expect(result.stdout).toContain(`Balance: ${mintAmount}`);
  });
});
