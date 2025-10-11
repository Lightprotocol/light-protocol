import { runCommand } from "@oclif/test";
import { expect } from "chai";
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

  it(`register mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`, async () => {
    const { stdout } = await runCommand([
      "create-token-pool",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("create-token-pool successful");
  });
});
