import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

describe("create-mint", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  let mintKeypair = Keypair.generate();
  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
    const lightWasm = await WasmFactory.getInstance();
    const rpc = await getTestRpc(lightWasm);

    await createTestSplMint(
      rpc,
      defaultSolanaWalletKeypair(),
      mintKeypair,
      mintAuthority,
    );
  });

  test
    .stdout({ print: true })
    .command(["register-mint", `--mint=${mintKeypair.publicKey.toBase58()}`])
    .it(
      `register mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("register-mint successful");
      },
    );
});
