import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createTestMint, requestAirdrop } from "../../helpers/helpers";

describe("create-token-account", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const mintKeypair = Keypair.generate();
  let mintAddress: PublicKey;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);
    mintAddress = await createTestMint(mintKeypair);
  });

  it("creates a token account for the payer", async () => {
    const { stdout } = await runCommand([
      "create-token-account",
      mintAddress.toBase58(),
    ]);
    expect(stdout).to.contain("create-token-account successful");
  });

  it("creates a token account with --owner", async () => {
    const otherOwner = Keypair.generate().publicKey;
    const { stdout } = await runCommand([
      "create-token-account",
      mintAddress.toBase58(),
      `--owner=${otherOwner.toBase58()}`,
    ]);
    expect(stdout).to.contain("create-token-account successful");
  });
});
