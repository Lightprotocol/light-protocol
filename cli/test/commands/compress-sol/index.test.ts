import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("compress-sol", () => {
  const keypair = defaultSolanaWalletKeypair() || Keypair.generate();
  const to = keypair.publicKey.toBase58();
  const amount = 0.5;

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(keypair.publicKey);
  });

  test
    .stdout()
    .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
    .it(`compress-sol ${amount} SOL to ${to}`, (ctx: any) => {
      expect(ctx.stdout).to.contain("mint-to successful");
    });
});
