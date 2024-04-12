import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("init-sol-pool", () => {
  const keypair = defaultSolanaWalletKeypair();

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(keypair.publicKey);
  });

  test
    .stdout({ print: true })
    .command(["init-sol-pool"])
    .it(`init-sol-pool`, (ctx) => {
      expect(ctx.stdout).to.contain("init-sol-pool successful");
    });
});
