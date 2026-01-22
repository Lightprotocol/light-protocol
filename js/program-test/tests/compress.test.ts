import { describe, it, assert, beforeAll, afterAll, expect } from "vitest";
import { Keypair } from "@solana/web3.js";
import {
  createLiteSVMRpc,
  newAccountWithLamports,
  NobleHasherFactory,
} from "../src";
import { compress, bn } from "@lightprotocol/stateless.js";

describe("compress with LiteSVM", () => {
  let rpc: any;
  let payer: Keypair;

  beforeAll(async () => {
    const lightWasm = await NobleHasherFactory.getInstance();
    rpc = await createLiteSVMRpc(lightWasm);

    // Create test account with lamports
    payer = await newAccountWithLamports(rpc, 10e9);
  });

  afterAll(() => {
    if (rpc && typeof rpc.clear === "function") {
      rpc.clear();
    }
  });

  it("should compress SOL", async () => {
    const compressAmount = 1e9;

    // Get pre-compress balance
    const preBalance = await rpc.getBalance(payer.publicKey);
    console.log("Pre-compress balance:", preBalance);

    // Compress SOL
    const signature = await compress(
      rpc,
      payer,
      compressAmount,
      payer.publicKey,
    );
    console.log("Compress signature:", signature);

    // Get post-compress balance
    const postBalance = await rpc.getBalance(payer.publicKey);
    console.log("Post-compress balance:", postBalance);

    // Get compressed accounts
    const compressedAccounts = await rpc.getCompressedAccountsByOwner(
      payer.publicKey,
    );
    console.log("Compressed accounts:", compressedAccounts);

    // Verify compression worked
    expect(compressedAccounts.items.length).toBeGreaterThan(0);

    // Verify compressed balance
    const compressedBalance = await rpc.getCompressedBalanceByOwner(
      payer.publicKey,
    );
    console.log("Compressed balance:", compressedBalance.toString());

    expect(compressedBalance.gte(bn(compressAmount))).toBe(true);
  });
});
