import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getSolanaRpcUrl } from "../../../src";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";
import { requestAirdrop } from "../../helpers/helpers";
import { createRpc } from "@lightprotocol/stateless.js";
describe("Get balance", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const payerKeypair = defaultSolanaWalletKeypair();

    const mintKeypair = Keypair.generate();
    await requestAirdrop(mintKeypair.publicKey);
    const mintAuthority = payerKeypair;

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
    return test
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

  async function createTestMint(payer: Keypair) {
    const rpc = createRpc(getSolanaRpcUrl());
    const { mint } = await createMint(rpc, payer, payer, 9, undefined, {
      commitment: "finalized",
    });
    return mint;
  }

  async function testMintTo(
    payer: Keypair,
    mintAddress: PublicKey,
    mintDestination: PublicKey,
    mintAuthority: Keypair,
    mintAmount: number,
  ) {
    const rpc = createRpc(getSolanaRpcUrl());
    const txId = await mintTo(
      rpc,
      payer,
      mintAddress,
      mintDestination,
      mintAuthority,
      mintAmount,
    );
    return txId;
  }
});
