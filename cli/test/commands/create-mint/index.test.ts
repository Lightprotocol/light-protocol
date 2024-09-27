import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";

describe("create-mint", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  
  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
  });

  it(`creates mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "create-mint",
      `--mint-authority=${mintAuthority.publicKey.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("create-mint successful");
  });
});
