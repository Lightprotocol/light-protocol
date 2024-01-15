import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import {
  AUTHORITY_ONE,
  airdropCompressedMINTSpl,
  airdropCompressedSol,
  airdropSol,
} from "@lightprotocol/zk.js";
import {
  getConfig,
  getLightProvider,
  readWalletFromFile,
} from "../../../src/utils/utils";
import { Connection } from "@solana/web3.js";

describe("accept-utxos", () => {
  before(async () => {
    await initTestEnvIfNeeded();
    const configWallet = readWalletFromFile();
    const config = getConfig();
    const connection = new Connection(config.solanaRpcUrl, "confirmed");
    const provider = await getLightProvider(true);
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
    await airdropCompressedSol({
      recipientPublicKey:
        "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      amount: 1,
      provider,
    });
    await airdropCompressedMINTSpl({
      recipientPublicKey:
        "MermoccL1uomVSnDrptQaeRTeiPQtJRgGx98gnm5o39X6RrWPLFKg9wf97yfqKVCwaDDrVCmaFwerWaQ6JSmmic",
      amount: 1,
      provider,
    });
  });

  test
    .stdout({ print: true })
    .command(["accept-utxos", "--token=SOL", "--all", "--localTestRpc"])
    .it("accept all SOL inbox utxos", (ctx: any) => {
      expect(ctx.stdout).to.contain("Accepted SOL inbox utxos successfully ✔");
    });

  test

    .stdout({ print: true })
    .command(["accept-utxos", "--token=USDC", "--all", "--localTestRpc"])
    .it("accept all USDC inbox utxos", (ctx: any) => {
      expect(ctx.stdout).to.contain(
        "Accepted USDC inbox utxos successfully ✔",
      );
    });
});
