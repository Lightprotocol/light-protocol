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

describe("transfer", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const payerKeypairPath = process.env.HOME + "/.config/solana/id.json";

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
      payerKeypair.publicKey,
      mintAuthority,
      mintAmount,
    );
  });

  it(`transfer ${
    mintAmount - 1
  } tokens to ${mintDestination.toBase58()} from ${mintKeypair.publicKey.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()} `, async () => {
    const { stdout } = await runCommand([
      "transfer",
      `--amount=${mintAmount - 1}`,
      `--fee-payer=${payerKeypairPath}`,
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--to=${mintDestination.toBase58()}`,
    ]);
    expect(stdout).to.contain("transfer successful");
  });
});
