import { expect, test } from "@oclif/test";
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

  test
    .stdout({ print: true })
      .command([
        "balance",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
        `--owner=${mintDestination.toBase58()}`,
      ])
      .it(
      `transfer ${mintAmount} tokens to ${mintDestination.toBase58()} from ${mintKeypair.publicKey.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()} `,
        (ctx: any) => {
          expect(ctx.stdout).to.contain("balance successful");
        },
      );
});
