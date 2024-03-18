import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { getPayer } from "../../../src";
import { requestAirdrop } from "../../helpers/helpers";

describe("create-mint", () => {
  test.it(async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(getPayer().publicKey);

    const mintAuthority = getPayer().publicKey.toBase58();
    const mintDecimals = 5;
    const mintKeypair = getPayer();

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
