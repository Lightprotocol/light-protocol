import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("compress-sol", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const keypair = defaultSolanaWalletKeypair() || Keypair.generate();
    await requestAirdrop(keypair.publicKey);
    const to = keypair.publicKey.toBase58();
    const amount = 0.5;

    return test
      .stdout({ print: true })
      .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
      .it(`compress-sol ${amount} SOL to ${to}`, (ctx: any) => {
        expect(ctx.stdout).to.contain("mint-to successful");
      });
  });
});
