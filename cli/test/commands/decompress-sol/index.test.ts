import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { createRpc, initSolOmnibusAccount } from "@lightprotocol/stateless.js";

describe("decompress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 200;
  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
    const rpc = createRpc();
    try {
      await initSolOmnibusAccount(rpc, keypair, keypair);
    } catch (e) {}
  });

  test
      .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
    .stdout({ print: true })
      .command(["decompress-sol", `--amount=${amount}`, `--to=${to}`])
      .it(`decompress-sol ${amount} SOL to ${to}`, (ctx: any) => {
      console.log(ctx.stdout);
        expect(ctx.stdout).to.contain("decompress-sol successful");
      });
});
