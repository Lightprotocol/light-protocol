import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getSolanaRpcUrl } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { requestAirdrop } from "../../helpers/helpers";
import { getTestRpc, TestRpc } from "@lightprotocol/stateless.js";

describe("decompress-sol", () => {
  const keypair = defaultSolanaWalletKeypair() || Keypair.generate();
  const to = keypair.publicKey.toBase58();
  const amount = 0.5;
  let rpc: TestRpc;

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(keypair.publicKey);
    rpc = await getTestRpc(getSolanaRpcUrl());
  });

  test
    .stdout()
    .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
    .command(["decompress-sol", `--amount=${amount}`, `--to=${to}`])
    .it(`decompress-sol ${amount} SOL to ${to}`, (ctx: any) => {
      expect(ctx.stdout).to.contain("decompress-sol successful");
    });
});
