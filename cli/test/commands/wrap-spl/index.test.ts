import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import {
  createTestSplMintWithPool,
  requestAirdrop,
} from "../../helpers/helpers";

describe("wrap-spl", () => {
  const payerKeypair = defaultSolanaWalletKeypair();

  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);
    await createTestSplMintWithPool(
      mintKeypair,
      mintAuthority,
      mintAmount,
      payerKeypair.publicKey,
    );
  });

  it(`wrap tokens`, async () => {
    // First unwrap some tokens to have SPL tokens available
    const { stdout: unwrapStdout } = await runCommand([
      "unwrap-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 1}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);
    console.log(unwrapStdout);

    // Then wrap SPL tokens back
    const { stdout } = await runCommand([
      "wrap-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 2}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("wrap-spl successful");
  });
});
