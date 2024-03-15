import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { getPayer, getSolanaRpcUrl } from "../../../src";
import { Connection, PublicKey } from "@solana/web3.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";
import { requestAirdrop } from "../../helpers/helpers";
describe("transfer", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(getPayer().publicKey);

    const mintAmount = 10;
    const mintAuthority = getPayer().publicKey;
    const mintDestination = getPayer().publicKey;
    const mintAddress = await createTestMint();
    await testMintTo(
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount * 2,
    );
    return test
      .stdout()
      .command([
        "transfer",
        `--amount=${mintAmount}`,
        `--fee-payer=${mintAuthority.toBase58()}`,
        `--mint=${mintAddress.toBase58()}`,
        `--to=${mintDestination.toBase58()}`,
      ])
      .it(
        `transfer ${mintAmount} tokens to ${mintDestination.toBase58()} from ${mintAddress.toBase58()}, fee-payer: ${mintAuthority.toBase58()} `,
        (ctx: any) => {
          expect(ctx.stdout).to.contain("mint-to successful");
        },
      );
  });

  async function createTestMint() {
    const connection = new Connection(getSolanaRpcUrl());
    const { mint, transactionSignature } = await createMint(
      connection,
      getPayer(),
      getPayer().publicKey,
      9,
      undefined,
      {
        commitment: "finalized",
      },
    );
    return mint;
  }

  async function testMintTo(
    mintAddress: PublicKey,
    mintDestination: PublicKey,
    mintAuthority: PublicKey,
    mintAmount: number,
  ) {
    const connection = new Connection(getSolanaRpcUrl());
    const txId = await mintTo(
      connection,
      getPayer(),
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount,
    );
    return txId;
  }
});
