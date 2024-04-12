import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import {
  defaultSolanaWalletKeypair,
  getPayer,
  getSolanaRpcUrl,
} from "../../../src";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createMint } from "@lightprotocol/compressed-token";
import { confirmTx, getTestRpc } from "@lightprotocol/stateless.js";
import { createTestMint, requestAirdrop } from "../../helpers/helpers";

describe("mint-to", () => {
  const mintKeypair = defaultSolanaWalletKeypair() || Keypair.generate();
  const mintAmount = 100;
  const mintAuthority = mintKeypair.publicKey.toBase58();
  const mintTo = mintAuthority;
  let mintAddress: PublicKey = PublicKey.default;

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(mintKeypair.publicKey);
    mintAddress = await createTestMint(mintKeypair);
  });

  test
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
