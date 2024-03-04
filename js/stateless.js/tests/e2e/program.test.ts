import { describe, it, expect, assert } from "vitest";
import { LightSystemProgram } from "../../src/programs/compressed-pda";
import { addMerkleContextToUtxo, createUtxo } from "../../src/state";
import {
  PAYER_KEYPAIR,
  byteArrayToKeypair,
} from "../../src/test-utils/init-accounts";

import {
  PublicKey,
  Connection,
  Transaction,
  VersionedMessage,
  TransactionMessage,
  VersionedTransaction,
  TransactionConfirmationStrategy,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  MockProof,
  createExecuteCompressedInstruction,
  UtxoWithBlinding,
} from "../../src/instruction/pack-nop-instruction";
import {
  COMPRESSED_LAMPORTS_MINIMUM,
  defaultTestStateTreeAccounts,
} from "../../src/constants";
import { TestRpc } from "../../src/test-utils/test-rpc";

// new todo
// - test case 1 utxo with lamports in - 2 out utxos -> send money
// - rename rpc fee to relay fee
// - test changelog event
// - test rpc get utxo by owner
// - clean up
// - script that does one transaction with constanst as amounts so that these are easy to change
// - readme (start test validator, in separate tab run script that does a transaction)

// TODO add tests for
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

  it("should match encoded inputs from test_execute_compressed_transactio from rust sdk", async () => {
    const keys = defaultTestStateTreeAccounts();
    const merkleTree = keys.merkleTree; /// TODO: replace with inited mt
    const queue = keys.stateNullifierQueue; /// TODO: replace with inited queue
    // const payer = PAYER_KEYPAIR;

    console.log("merkleTree", merkleTree.toBase58());
    console.log("queue", queue.toBase58());

    const payer = byteArrayToKeypair([
      122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
      158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132,
      236, 227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163,
      243, 41, 118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36,
      251, 130,
    ]);

    console.log("payer", payer.publicKey.toBase58());

    const in_utxos: UtxoWithBlinding[] = [
      {
        owner: payer.publicKey,
        //@ts-ignore
        lamports: new BN(0),
        blinding: new Array(32).fill(1),
        //@ts-ignore
        data: null,
      },
    ];
    const out_utxos = [
      { owner: payer.publicKey, lamports: new BN(0), data: null },
    ];

    const recipient = payer.publicKey;
    const proof_mock: MockProof = {
      a: Array.from({ length: 32 }, () => 0),
      b: Array.from({ length: 64 }, () => 0),
      c: Array.from({ length: 32 }, () => 0),
    };

    const ix = await createExecuteCompressedInstruction(
      payer.publicKey,
      in_utxos,
      //@ts-ignore // lamports is not a BN
      out_utxos,
      [merkleTree],
      [queue],
      [merkleTree],
      [0],
      proof_mock
    );
    const ixs = [ix];
    const connection = new Connection("http://localhost:8899", "confirmed");

    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash();
    const balancePayer = await connection.getBalance(payer.publicKey);
    const balanceRecipient = await connection.getBalance(recipient);
    console.log("balance", balancePayer, balanceRecipient);

    const sig = await connection.requestAirdrop(payer.publicKey, 2e9);

    const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
      signature: sig,
      blockhash,
      lastValidBlockHeight,
    };
    console.log("confirming...", sig);
    await connection.confirmTransaction(
      transactionConfirmationStrategy,
      "confirmed"
    );
    console.log("sig", sig, "payer", payer.publicKey.toBase58());
    const balancePayerAfterAirdrop = await connection.getBalance(
      payer.publicKey,
      "confirmed"
    );
    console.log("balancePayerAfterAirdrop", balancePayerAfterAirdrop);

    // throw new Error("stop here");
    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    tx.message.compiledInstructions[0].accountKeyIndexes.forEach((index, _) => {
      console.log(
        `Account ${index}: ${tx.message.staticAccountKeys[
          index
        ].toBase58()} - Signer: ${tx.message.isAccountSigner(index)}`
      );
    });
    tx.sign([payer]);

    console.log("tx", tx.signatures, "\n", tx.message.getAccountKeys());
    const txid = await connection.sendTransaction(tx);

    console.log(
      `https://explorer.solana.com/tx/${txid}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`
    );
    expect(txid).toBeTruthy();

    const transactionConfirmationStrategy0: TransactionConfirmationStrategy = {
      signature: txid,
      blockhash,
      lastValidBlockHeight,
    };
    await connection.confirmTransaction(
      transactionConfirmationStrategy0,
      "confirmed"
    );
    const testRpc = new TestRpc({
      connection,
    });

    const indexedEvents = await testRpc.getIndexedTransactions();
    console.log("indexedEvents", indexedEvents);
    console.log("indexedEvents in utxos", indexedEvents[0].inUtxos);
    console.log("indexedEvents out utxos", indexedEvents[0].outUtxos[0]);
    assert.equal(indexedEvents.length, 1);
    assert.equal(indexedEvents[0].inUtxos.length, 1);
    assert.equal(indexedEvents[0].outUtxos.length, 1);
    assert.equal(indexedEvents[0].outUtxos[0].lamports, 0);
    assert.equal(
      indexedEvents[0].outUtxos[0].owner,
      payer.publicKey.toBase58()
    );
    assert.equal(indexedEvents[0].outUtxos[0].data, null);
  });

  it.skip("should build ix and send to chain successfully", async () => {
    const keys = defaultTestStateTreeAccounts();
    const merkleTree = keys.merkleTree; /// TODO: replace with inited mt
    const queue = keys.stateNullifierQueue; /// TODO: replace with inited queue
    const payer = PAYER_KEYPAIR;

    const recipient = PublicKey.unique();
    const inputState = [
      //   addMerkleContextToUtxo(
      //     createUtxo(payer.publicKey, 1_000_000_000n),
      //     0n,
      //     merkleTree,
      //     0,
      //     queue
      //   ),
    ];
    const outputState = [
      //   createUtxo(recipient, 120_000_000n),
      //   createUtxo(payer.publicKey, 880_000_000n),
      createUtxo(recipient, 0n),
      createUtxo(payer.publicKey, 0n),
    ];
    // const mockProof = placeholderValidityProof();
    const mockProof: MockProof = {
      a: Array.from({ length: 32 }, (_, i) => i),
      b: Array.from({ length: 64 }, (_, i) => i),
      c: Array.from({ length: 32 }, (_, i) => i),
    };

    const ix = await createExecuteCompressedInstruction(
      payer.publicKey,
      inputState,
      outputState,
      [], //[merkleTree],
      [], // [queue],
      [merkleTree],
      [],
      mockProof
    );

    const ixs = [ix];
    const connection = new Connection("http://localhost:8899", "confirmed");

    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash();
    const balancePayer = await connection.getBalance(payer.publicKey);
    const balanceRecipient = await connection.getBalance(recipient);
    console.log("balance", balancePayer, balanceRecipient);

    const sig = await connection.requestAirdrop(payer.publicKey, 2e9);

    const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
      signature: sig,
      blockhash,
      lastValidBlockHeight,
    };
    console.log("confirming...", sig);
    await connection.confirmTransaction(
      transactionConfirmationStrategy,
      "confirmed"
    );
    console.log("sig", sig, "payer", payer.publicKey.toBase58());
    const balancePayerAfterAirdrop = await connection.getBalance(
      payer.publicKey,
      "confirmed"
    );
    console.log("balancePayerAfterAirdrop", balancePayerAfterAirdrop);

    // throw new Error("stop here");
    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    tx.message.compiledInstructions[0].accountKeyIndexes.forEach((index, _) => {
      console.log(
        `Account ${index}: ${tx.message.staticAccountKeys[
          index
        ].toBase58()} - Signer: ${tx.message.isAccountSigner(index)}`
      );
    });
    tx.sign([payer]);

    console.log("tx", tx.signatures, "\n", tx.message.getAccountKeys());
    const txid = await connection.sendTransaction(tx);

    console.log(
      `https://explorer.solana.com/tx/${txid}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`
    );
    expect(txid).toBeTruthy();
  });
});
