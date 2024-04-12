import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getPayer } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
describe("create-mint", () => {
  const mintDecimals = 5;
  const mintKeypair = defaultSolanaWalletKeypair();

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(mintKeypair.publicKey);
  });

  test
    .stdout()
    .command([
      "create-mint",
      `--mint-decimals=${mintDecimals}`,
    ])
    .it(`create mint for ${mintKeypair.publicKey.toBase58()} with 2 decimals`, (ctx: any) => {

      expect(ctx.stdout).to.contain("create-mint successful");
    });
});
