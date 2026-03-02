import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";
import { getSolanaRpcUrl, getIndexerUrl, getProverUrl } from "../../../src/utils/utils";

describe("create-interface-pda", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  let mintKeypair = Keypair.generate();
  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
    const rpc = createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());
    const payer = defaultSolanaWalletKeypair();
    await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);
  });

  it(`create interface PDA for mint`, async () => {
    const { stdout } = await runCommand([
      "create-interface-pda",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("create-interface-pda successful");
  });
});
