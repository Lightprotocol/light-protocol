import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { createTestMint, requestAirdrop } from "../../helpers/helpers";

describe("mint-to", () => {
  let mintAmount: number = 100;
  let mintAuthorityPath = process.env.HOME + "/.config/solana/id.json";
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();

  let mintKeypair = Keypair.generate();
  let mintAddress = mintKeypair.publicKey;

  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
    await createTestMint(mintKeypair);
  });

  it(`mints ${mintAmount} tokens to ${mintAuthority.publicKey.toBase58()} from mint: ${mintAddress.toBase58()} with authority ${mintAuthority.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "mint-to",
      `--amount=${mintAmount}`,
      `--mint=${mintAddress.toBase58()}`,
      `--mint-authority=${mintAuthorityPath}`,
      `--to=${mintAuthority.publicKey.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("mint-to successful");
  });
});
