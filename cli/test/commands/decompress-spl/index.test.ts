import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import {
  createTestMint,
  requestAirdrop,
  testMintTo,
} from "../../helpers/helpers";

describe("decompress-spl", () => {
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

  test
    .stdout({ print: true })
    .command([
      "decompress-spl",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
      `--amount=${mintAmount - 1}`,
      `--to=${payerKeypair.publicKey.toBase58()}`,
    ])
    .it(
      `decompress ${
        mintAmount - 1
      } tokens to ${payerKeypair.publicKey.toBase58()} from ${payerKeypair.publicKey.toBase58()}`,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("decompress-spl successful");
      },
    );
});
