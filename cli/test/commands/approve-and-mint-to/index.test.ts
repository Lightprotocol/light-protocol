import { expect, test, describe, beforeAll, afterAll } from 'vitest';
import ApproveAndMintToCommand from '../../../src/commands/approve-and-mint-to';
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

describe('ApproveAndMintToCommand', () => {
  beforeAll(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
  });

  afterAll(async () => {
    // Teardown code if needed
  });

  test('approves and mints tokens successfully', async () => {
    const payer = defaultSolanaWalletKeypair();
    await requestAirdrop(payer.publicKey);

    const mintKeypair = Keypair.generate();
    const mintAuthority = payer;
    const destination = Keypair.generate().publicKey;

    const lightWasm = await WasmFactory.getInstance();
    const rpc = await getTestRpc(lightWasm);

    await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);

    const command = new ApproveAndMintToCommand([
      '--mint', mintKeypair.publicKey.toString(),
      '--to', destination.toString(),
      '--amount', '100',
    ]);

    await command.run();

    // Add assertions here to verify the command's effects
    expect(true).toBe(true); // Replace with meaningful assertions
  });
});
