import { expect, test } from "@oclif/test";
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

  const mintAmount = 10;
  const mintDestination = Keypair.generate().publicKey;
  let mintAddress: PublicKey = PublicKey.default;

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(mintKeypair.publicKey);
    mintAddress = await createTestMint(payerKeypair);
    await testMintTo(
      payerKeypair,
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount,
    );
  });

  test
    .stdout()
    .command([
      "balance",
      `--mint=${mintAddress.toBase58()}`,
      `--owner=${mintDestination.toBase58()}`,
    ])
    .it(
      `runs balance --mint=${mintAddress.toBase58()} --owner=${mintDestination.toBase58()}`,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("balance successful");
      },
    );
});
