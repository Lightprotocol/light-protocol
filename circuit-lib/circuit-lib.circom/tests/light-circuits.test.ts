import {
  confirmConfig,
  functionalCircuitTest,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
} from "@lightprotocol/zk.js";
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair as SolanaKeypair } from "@solana/web3.js";

describe("verifier_program", () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  before(async () => {
    try {
      const provider = new anchor.AnchorProvider(
          new Connection("http://127.0.0.1:8899"),
        new anchor.Wallet(SolanaKeypair.generate()),
        confirmConfig,
      );
      anchor.setProvider(provider);
    } catch (error) {
      console.log("expected local test validator to be running");
      process.exit();
    }
  });

  it("Test functional circuit 2 in 2 out", async () => {
    await functionalCircuitTest(false, IDL_LIGHT_PSP2IN2OUT);
  });

  it("Test functional circuit 10 in 2 out", async () => {
    await functionalCircuitTest(false, IDL_LIGHT_PSP10IN2OUT);
  });

  it("Test functional circuit 4 in 4 out + connecting hash", async () => {
    await functionalCircuitTest(true, IDL_LIGHT_PSP4IN4OUT_APP_STORAGE);
  });
});
