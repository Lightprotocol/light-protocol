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

describe("Get balance", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
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
      mintDestination,
      mintAuthority,
      mintAmount,
    );
  });

  it(`check token balance`, async () => {
    const { stdout } = await runCommand([
      "token-balance",
      `--mint=${mintAddress.toBase58()}`,
      `--owner=${mintDestination.toBase58()}`,
    ]);
    expect(stdout).to.contain("Light token account balance:");
    expect(stdout).to.contain("Compressed light token balance:");
    expect(stdout).to.contain("Total balance:");
  });
});
