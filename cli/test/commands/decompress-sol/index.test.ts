import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("decompress-sol", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const keypair = defaultSolanaWalletKeypair() || Keypair.generate();
    await requestAirdrop(keypair.publicKey);
    const to = keypair.publicKey.toBase58();
    const amount = 0.5;
    return test
      .stdout()
      .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
      .command(["decompress-sol", `--amount=${amount}`, `--to=${to}`])
      .it(`decompress-sol ${amount} SOL to ${to}`, (ctx: any) => {
        expect(ctx.stdout).to.contain("decompress-sol successful");
      });
  });
});
