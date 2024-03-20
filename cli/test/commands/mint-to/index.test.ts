import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import {
  defaultSolanaWalletKeypair,
  getPayer,
  getSolanaRpcUrl,
} from "../../../src";
import { Connection, Keypair } from "@solana/web3.js";
import { createMint } from "@lightprotocol/compressed-token";
import { confirmTx } from "@lightprotocol/stateless.js";
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
    const connection = new Connection(getSolanaRpcUrl(), "finalized");
    const { mint, transactionSignature } = await createMint(
      connection,
      getPayer(),
      getPayer().publicKey,
      9,
    );
    await confirmTx(connection, transactionSignature);
    return mint;
  }
});
