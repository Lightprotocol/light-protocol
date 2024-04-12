import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { createRpc, initSolOmnibusAccount } from "@lightprotocol/stateless.js";

describe("compress-sol", () => {
  const keypair = defaultSolanaWalletKeypair();
  const to = keypair.publicKey.toBase58();
  const amount = 500;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
    const rpc = createRpc();
    try {
      await initSolOmnibusAccount(rpc, keypair, keypair);
    } catch (e) {}
  });

  test
    .stdout({ print: true })
      .command(["compress-sol", `--amount=${amount}`, `--to=${to}`])
    .it(`compress-sol ${amount} lamports to ${to}`, (ctx) => {
      expect(ctx.stdout).to.contain("compress-sol successful");
  });
});
