import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getSolanaRpcUrl } from "../../../src";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";
import { requestAirdrop } from "../../helpers/helpers";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
describe("transfer", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const payerKeypair = defaultSolanaWalletKeypair();

    const mintKeypair = Keypair.generate();
    await requestAirdrop(mintKeypair.publicKey);
    const mintAuthority = payerKeypair.publicKey;

    const mintAmount = 10;
    const mintDestination = Keypair.generate().publicKey;
    const mintAddress = await createTestMint(payerKeypair);

    await testMintTo(
      payerKeypair,
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount,
    );
    const encodedPayer = bs58.encode(payerKeypair.secretKey);
    return test
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

  async function createTestMint(payer: Keypair) {
    const connection = new Connection(getSolanaRpcUrl());
    const { mint } = await createMint(
      connection,
      payer,
      payer.publicKey,
      9,
      undefined,
      {
        commitment: "finalized",
      },
    );
    return mint;
  }

  async function testMintTo(
    payer: Keypair,
    mintAddress: PublicKey,
    mintDestination: PublicKey,
    mintAuthority: PublicKey | Keypair,
    mintAmount: number,
  ) {
    const connection = new Connection(getSolanaRpcUrl());
    const txId = await mintTo(
      connection,
      payer,
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount,
    );
    return txId;
  }
});
