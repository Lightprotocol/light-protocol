import { runCommand } from "@oclif/test";
import { expect } from "chai";
import { before } from "mocha";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

describe("mint-to", () => {
  let mintAmount: number = 100;
  /// authority is also the feepayer, and mint-to recipient
  let mintAuthorityPath = process.env.HOME + "/.config/solana/id.json";
  let mintAuthority: Keypair = defaultSolanaWalletKeypair();

  let mintKeypair = Keypair.generate();
  let mintAddress = mintKeypair.publicKey;

  before(async () => {
    await initTestEnvIfNeeded({ indexer: true, prover: true });
    await requestAirdrop(mintAuthority.publicKey);
    const lightWasm = await WasmFactory.getInstance();
    const rpc = await getTestRpc(lightWasm);
    await createTestSplMint(rpc, mintAuthority, mintKeypair, mintAuthority);
  });

  it(`approve-and-mint-to ${mintAmount} tokens to ${mintAuthority.publicKey.toBase58()} from mint: ${mintAddress.toBase58()} with authority ${mintAuthority.publicKey.toBase58()}`, async () => {
    // First create the token pool
    const { stdout: poolStdout } = await runCommand([
      "create-token-pool",
      `--mint=${mintKeypair.publicKey.toBase58()}`,
    ]);
    console.log(poolStdout);

    // Then approve and mint
    const { stdout } = await runCommand([
      "approve-and-mint-to",
      `--amount=${mintAmount}`,
      `--mint=${mintAddress.toBase58()}`,
      `--mint-authority=${mintAuthorityPath}`,
      `--to=${mintAuthority.publicKey.toBase58()}`,
    ]);
    expect(stdout).to.contain("approve-and-mint-to successful");
  });
});
