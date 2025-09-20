import { runCommand } from "@oclif/test";
import { expect } from "chai";
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

  before(async () => {
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

  it(`check balance of ${mintAmount} tokens for ${mintDestination.toBase58()} from mint ${mintKeypair.publicKey.toBase58()}`, async () => {
    const { stdout } = await runCommand([
      "token-balance",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--owner=${mintDestination.toBase58()}`,
    ]);
    expect(stdout).to.contain("Balance:");
  });
});
