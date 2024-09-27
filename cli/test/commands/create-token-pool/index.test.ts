import { expect, describe, it, beforeAll } from 'vitest';
import { runCommand } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { Keypair } from "@solana/web3.js";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

describe("create-token-pool", () => {
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();
  let mintKeypair = Keypair.generate();
  
  beforeAll(async () => {
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

  it(`registers mint for mintAuthority: ${mintAuthority.publicKey.toBase58()}`, async () => {
    const result = await runCommand([
      "create-token-pool",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);

    expect(result.error).toBeUndefined();
    expect(result.stdout).toContain("create-token-pool successful");
  });
});
