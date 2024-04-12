import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
describe("create-mint", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
  });

  /// TODO: add flags once the command is being executed
  test
    .stdout()
    .command(["create-mint"])
    .it(
      `create mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("create-mint successful");
      },
    );
});
