import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair, PublicKey } from "@solana/web3.js";
import {
  createTestMint,
  requestAirdrop,
  testMintTo,
} from "../../helpers/helpers";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
describe("transfer", () => {
  const payerKeypair = defaultSolanaWalletKeypair();
  const mintKeypair = Keypair.generate();
  const mintAuthority = payerKeypair;
  const mintAmount = 10;
  const mintDestination = Keypair.generate().publicKey;
  let mintAddress: PublicKey = PublicKey.default;
  const encodedPayer = bs58.encode(payerKeypair.secretKey);

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
      "transfer",
      `--amount=${mintAmount - 1}`,
      `--fee-payer=${encodedPayer}`,
      `--mint=${mintAddress.toBase58()}`,
      `--to=${mintDestination.toBase58()}`,
    ])
    .it(
      `transfer ${mintAmount} tokens to ${mintDestination.toBase58()} from ${mintAddress.toBase58()}, fee-payer: ${payerKeypair.publicKey.toBase58()} `,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("mint-to successful");
      },
    );
});
