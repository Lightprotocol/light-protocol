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
    .stderr()
    .command(["init-sol-pool"])
    .catch((ctx) => {
      expect(ctx.message).to.contain(
        "Failed to init-sol-pool!\nAlready inited.",
      );
    })
    .it(
      "expects init-sol-pool command to fail due to already initialized pool",
    );
});
