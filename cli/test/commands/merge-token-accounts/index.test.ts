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
import { rpc } from "../../../src/utils/utils";

describe("merge-token-accounts", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const payerKeypairPath = process.env.HOME + "/.config/solana/id.json";

  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;

  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);
    await createTestMint(mintKeypair);

    for (let i = 0; i < 3; i++) {
      await testMintTo(
        payerKeypair,
        mintKeypair.publicKey,
        payerKeypair.publicKey,
        mintAuthority,
        mintAmount,
      );
    }
  });

  it(`merges token accounts for mint ${mintKeypair.publicKey.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "merge-token-accounts",
      `--fee-payer=${payerKeypairPath}`,
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("Token accounts merged successfully");

    const accounts = await rpc().getCompressedTokenAccountsByOwner(
      payerKeypair.publicKey,
      { mint: mintKeypair.publicKey },
    );
    expect(accounts.items.length).toBe(1);
    expect(accounts.items[0].parsed.amount.toNumber()).toBe(mintAmount * 3);
  });
});
