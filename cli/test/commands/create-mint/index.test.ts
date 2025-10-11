import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
describe("create-mint", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
  });

  it(`create mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`, async () => {
    const { stdout } = await runCommand([
      "create-mint",
      `--mint-authority=${mintAuthority.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("create-mint successful");
  });
});
