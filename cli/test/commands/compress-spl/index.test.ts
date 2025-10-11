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

describe("compress-spl", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  /// TODO: add test case for separate fee-payer
  const payerKeypairPath = process.env.HOME + "/.config/solana/id.json";

  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;

  before(async () => {
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

  it(`compress ${
    mintAmount - 2
  } tokens to ${payerKeypair.publicKey.toBase58()} from ${payerKeypair.publicKey.toBase58()}`, async () => {
    // First decompress some tokens to have them available
    const { stdout: decompressStdout } = await runCommand([
      "decompress-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 1}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);
    console.log(decompressStdout);

    // Then compress tokens
    const { stdout } = await runCommand([
      "compress-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 2}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("compress-spl successful");
  });
});
