import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getPayer } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
describe("create-mint", () => {
  const mintDecimals = 5;
  const mintKeypair = defaultSolanaWalletKeypair();
  const mintSecretKey = bs58.encode(mintKeypair.secretKey);
  const mintAuthority = Keypair.generate();

  before(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(mintAuthority.publicKey);
  });

  test
    .stdout()
    .command([
      "create-mint",
      `--mint-decimals=${mintDecimals}`,
      `--mint-authority=${mintAuthority.publicKey.toBase58()}`,
      `--mint-keypair=${mintSecretKey}`,
    ])
    .it(`create mint for ${mintAuthority} with 2 decimals`, (ctx: any) => {
      expect(ctx.stdout).to.contain("create-mint successful");
    });
});
