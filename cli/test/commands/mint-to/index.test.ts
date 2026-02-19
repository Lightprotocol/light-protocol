import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { before } from "mocha";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createTestMint, requestAirdrop } from "../../helpers/helpers";

describe("mint-to", () => {
  let mintAmount: number = 100;
  /// authority is also the feepayer, and mint-to recipient
  let mintAuthorityPath = process.env.HOME + "/.config/solana/id.json";
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();

  let mintKeypair = Keypair.generate();
  let mintAddress: PublicKey;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
    mintAddress = await createTestMint(mintKeypair);
  });

  it(`mint-to tokens`, async () => {
    const { stdout } = await runCommand([
      "mint-to",
      `--amount=${mintAmount}`,
      `--mint=${mintAddress.toBase58()}`,
      `--mint-authority=${mintAuthorityPath}`,
      `--to=${mintAuthority.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("mint-to successful");
  });
});
