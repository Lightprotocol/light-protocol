import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  FEE_ASSET,
  Account,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  Utxo,
  VerifierZero,
  SolMerkleTree,
  VerifierTwo,
  Verifier,
  VerifierOne,
  Action,
  functionalCircuitTest,
} from "light-sdk";
import * as anchor from "@coral-xyz/anchor";
import { assert, expect } from "chai";
import { Connection, Keypair as SolanaKeypair } from "@solana/web3.js";
const circomlibjs = require("circomlibjs");

describe("verifier_program", () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  before(async () => {
    try {
      const provider = new anchor.AnchorProvider(
        await new Connection("http://127.0.0.1:8899"),
        new anchor.Wallet(SolanaKeypair.generate()),
        confirmConfig
      );
      await anchor.setProvider(provider);
    } catch (error) {
      console.log("expected local test validator to be running");
      process.exit();
    }
  });

  it("Test functional circuit 2 in 2 out", async () => {
    await functionalCircuitTest(new VerifierZero());
  });

  it("Test functional circuit 10 in 2 out", async () => {
    await functionalCircuitTest(new VerifierOne());
  });

  it("Test functional circuit 4 in 4 out + connecting hash", async () => {
    await functionalCircuitTest(new VerifierTwo(), true);
  });
});
