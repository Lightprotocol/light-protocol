import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import {
  createTestSplMintWithPool,
  requestAirdrop,
} from "../../helpers/helpers";

describe("unwrap-spl", () => {
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

  it(`unwrap tokens`, async () => {
    const { stdout } = await runCommand([
      "unwrap-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 1}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("unwrap-spl successful");
  });
});
