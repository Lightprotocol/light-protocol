import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair, PublicKey } from "@solana/web3.js";
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
  let mintAddress: PublicKey;

  const mintAmount = 10;
  const mintDestination = Keypair.generate().publicKey;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);
    mintAddress = await createTestMint(mintKeypair);

    await testMintTo(
      payerKeypair,
      mintAddress,
      payerKeypair.publicKey,
      mintAuthority,
      mintAmount,
    );
  });

  it(`transfer tokens`, async () => {
    const { stdout } = await runCommand([
      "transfer",
      `--amount=${mintAmount - 1}`,
      `--fee-payer=${payerKeypairPath}`,
      `--mint=${mintAddress.toBase58()}`,
      `--to=${mintDestination.toBase58()}`,
    ]);
    expect(stdout).to.contain("transfer successful");
  });
});
