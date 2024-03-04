import { describe, it, expect } from "vitest";
import { LightSystemProgram } from "../../src/programs/compressed-pda";
import { addMerkleContextToUtxo, createUtxo } from "../../src/state";
import { PAYER_KEYPAIR } from "../../src/test-utils/init-accounts";
import {
  PublicKey,
  Connection,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { placeholderValidityProof } from "../../src/instruction/validity-proof";
import { createExecuteCompressedInstruction } from "../../src/instruction/pack-nop-instruction";

// TODO add tests for
// - double spend check
// - sumcheck fail check
// - invalid tx signer
// - asserting emitted events..
// - repeat: sending fetching balance, sending more
// its not pretty but should work
describe("Program test", () => {
  // TODO: remove
  it("should pass", () => {
    console.log(
      "Testing vitest setup here!",
      LightSystemProgram.programId.toBase58()
    );
    expect(true).toBe(true);
  });

  it("should build ix and send to chain successfully", async () => {
    const merkleTree = PublicKey.unique(); /// TODO: replace with inited mt
    const queue = PublicKey.unique(); /// TODO: replace with inited queue
    const payer = PAYER_KEYPAIR;
    const recipient = PublicKey.unique();
    const inputState = [
      addMerkleContextToUtxo(
        createUtxo(payer.publicKey, 100n),
        0n,
        merkleTree,
        0,
        queue
      ),
    ];
    const outputState = [
      createUtxo(recipient, 10n),
      createUtxo(payer.publicKey, 90n),
    ];
    const mockProof = placeholderValidityProof();

    const ix = await createExecuteCompressedInstruction(
      payer.publicKey,
      inputState,
      outputState,
      [merkleTree],
      [queue],
      [merkleTree],
      [0, 0], // 2 outputs
      mockProof
    );
    const tx = new Transaction().add(ix);
    /// connect to test validator
    const connection = new Connection("http://localhost:8899");
    const sig = await sendAndConfirmTransaction(connection, tx, [payer], {
      commitment: "single",
    });
    console.log("sig", sig); /// actually assert
    expect(sig).toBeTruthy();
  });
});
