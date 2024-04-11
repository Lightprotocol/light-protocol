import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("compress-sol", () => {
  test.it(async () => {
    console.log("compress-sol");
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    console.log("compress-sol2");
    const keypair = defaultSolanaWalletKeypair() || Keypair.generate();
    await requestAirdrop(keypair.publicKey);
    console.log("compress-sol3");
    const to = keypair.publicKey.toBase58();
    const amount = 0.5;

    console.log("test", test.stdout);
    test
      .stdout()
      .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
      .it(`compress-sol ${amount} SOL to ${to}`, (ctx) => {
        expect(ctx.stdout).to.contain("compress-sol successful");
      });
  });
});
