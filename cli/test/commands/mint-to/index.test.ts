import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import {
  defaultSolanaWalletKeypair,
  getPayer,
  getSolanaRpcUrl,
} from "../../../src";
import { Connection, Keypair } from "@solana/web3.js";
import { createMint } from "@lightprotocol/compressed-token";
import { confirmTx, getTestRpc } from "@lightprotocol/stateless.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("mint-to", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const mintKeypair = defaultSolanaWalletKeypair() || Keypair.generate();
    await requestAirdrop(mintKeypair.publicKey);
    const mintAmount = 100;
    const mintAuthority = mintKeypair.publicKey.toBase58();
    const mintTo = mintAuthority;
    const mintAddress = await createTestMint();
    return test
      .stdout()
      .command([
        "mint-to",
        `--amount=${mintAmount}`,
        `--mint=${mintAddress}`,
        `--mint-authority=${mintAuthority}`,
        `--to=${mintTo}`,
      ])
      .it(
        `mint-to ${mintAmount} tokens to ${mintTo} from ${mintAddress} with authority ${mintAuthority}`,
        (ctx: any) => {
          expect(ctx.stdout).to.contain("mint-to successful");
        },
      );
  });

  async function createTestMint() {
    const rpc = await getTestRpc(
      getSolanaRpcUrl(),
      undefined,
      undefined,
      undefined,
    );

    const { mint, transactionSignature } = await createMint(
      rpc,
      await getPayer(),
      await getPayer(),
      9,
    );
    await confirmTx(rpc, transactionSignature);
    return mint;
  }
});
