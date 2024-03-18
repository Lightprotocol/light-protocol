import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair, getPayer } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";

describe("create-mint", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    const mintDecimals = 5;
    const mintKeypair = defaultSolanaWalletKeypair() || Keypair.generate();
    const mintAuthority = mintKeypair.publicKey.toBase58();
    await requestAirdrop(mintKeypair.publicKey);
    return test
      .stdout()
      .command([
        "create-mint",
        `--mint-decimals=${mintDecimals}`,
        `--mint-authority=${mintAuthority}`,
        `--mint-keypair=${mintKeypair}`,
      ])
      .it(`create mint for ${mintAuthority} with 2 decimals`, (ctx: any) => {
        expect(ctx.stdout).to.contain("create-mint successful");
      });
  });
});
