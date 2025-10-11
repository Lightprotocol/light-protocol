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
import { rpc } from "../../../src/utils/utils";

describe("merge-token-accounts", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const payerKeypairPath = process.env.HOME + "/.config/solana/id.json";

  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;

  const mintAmount = 10;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(payerKeypair.publicKey);
    await createTestMint(mintKeypair);

    // Create multiple token accounts
    for (let i = 0; i < 3; i++) {
      await testMintTo(
        payerKeypair,
        mintKeypair.publicKey,
        payerKeypair.publicKey,
        mintAuthority,
        mintAmount,
      );
    }
  });

  it(`merge token accounts for mint ${mintKeypair.publicKey.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()} `, async () => {
    const { stdout } = await runCommand([
      "merge-token-accounts",
      `--fee-payer=${payerKeypairPath}`,
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("Token accounts merged successfully");

    // Verify that accounts were merged
    const accounts = await rpc().getCompressedTokenAccountsByOwner(
      payerKeypair.publicKey,
      { mint: mintKeypair.publicKey },
    );
    expect(accounts.items.length).to.equal(1);
    expect(accounts.items[0].parsed.amount.toNumber()).to.equal(mintAmount * 3);
  });
});
