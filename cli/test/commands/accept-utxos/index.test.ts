import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import {
  AUTHORITY_ONE,
  airdropShieldedMINTSpl,
  airdropShieldedSol,
  airdropSol,
} from "@lightprotocol/zk.js";
import { getConfig, readWalletFromFile } from "../../../src/utils/utils";
import { Connection } from "@solana/web3.js";

describe("accept-utxos", () => {
  before(async () => {
    await initTestEnvIfNeeded();
    const configWallet = readWalletFromFile();
    const config = getConfig();
    const connection = new Connection(config.rpcUrl, "confirmed");
    await airdropSol({
      connection,
      lamports: 10e9,
      recipientPublicKey: configWallet.publicKey,
    });
    await airdropSol({
      connection,
      lamports: 10e9,
      recipientPublicKey: AUTHORITY_ONE,
    });
    await airdropShieldedSol({
      recipientPublicKey:
        "DVTtJhghZU1hBEbCci4RDpRP1K1eEHZXyYognZ4BNiCBaM8WenG3o6v8CNcKTRD7fVUsSTtae8hU5To1ogrGQDw",
      amount: 1,
    });
    await airdropShieldedMINTSpl({
      recipientPublicKey:
        "DVTtJhghZU1hBEbCci4RDpRP1K1eEHZXyYognZ4BNiCBaM8WenG3o6v8CNcKTRD7fVUsSTtae8hU5To1ogrGQDw",
      amount: 1,
    });
  });

  test
    .stdout({ print: true })
    .command(["accept-utxos", "--token=SOL", "--all", "--localTestRelayer"])
    .it("accept all SOL inbox utxos", (ctx: any) => {
      expect(ctx.stdout).to.contain("Accepted SOL inbox utxos successfully ✔");
    });

  test

    .stdout({ print: true })
    .command(["accept-utxos", "--token=USDC", "--all", "--localTestRelayer"])
    .it("accept all USDC inbox utxos", (ctx: any) => {
      expect(ctx.stdout).to.contain("Accepted USDC inbox utxos successfully ✔");
    });
});
