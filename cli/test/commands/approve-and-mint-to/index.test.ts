import { expect, test } from "@oclif/test";
import { before } from "mocha";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { defaultSolanaWalletKeypair } from "../../../src";
import { Keypair } from "@solana/web3.js";
import { createTestSplMint, requestAirdrop } from "../../helpers/helpers";
import { getTestRpc } from "@lightprotocol/stateless.js";

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
    const rpc = await getTestRpc();
    await createTestSplMint(rpc, mintAuthority, mintKeypair, mintAuthority);
  });

  test
    .command(["register-mint", `--mint=${mintKeypair.publicKey.toBase58()}`])
    .stdout({ print: true })
    .command([
      "approve-and-mint-to",
      `--amount=${mintAmount}`,
      `--mint=${mintAddress.toBase58()}`,
      `--mint-authority=${mintAuthorityPath}`,
      `--to=${mintAuthority.publicKey.toBase58()}`,
    ])
    .it(
      `approve-and-mint-to ${mintAmount} tokens to ${mintAuthority.publicKey.toBase58()} from mint: ${mintAddress.toBase58()} with authority ${mintAuthority.publicKey.toBase58()}`,
      (ctx: any) => {
        expect(ctx.stdout).to.contain("approve-and-mint-to successful");
      },
    );
});
