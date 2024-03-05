#!/usr/bin/env ts-node
import { Utxo, bn } from "../src/state";
import { byteArrayToKeypair } from "../src/test-utils/init-accounts";

import {
  Connection,
  TransactionMessage,
  VersionedTransaction,
  Keypair,
} from "@solana/web3.js";

import {
  MockProof,
  createExecuteCompressedInstruction,
  UtxoWithBlinding,
} from "../src/instruction/pack-nop-instruction";
import { FIELD_SIZE, defaultTestStateTreeAccounts } from "../src/constants";
import { confirmTx, sendAndConfirmTx } from "../src/test-utils";

const FIXED_PAYER = byteArrayToKeypair([
  122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
  158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
  227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
  118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
]);

// <254bit ,
const rndMockedBlinding = () =>
  bn(Array.from(crypto.getRandomValues(new Uint8Array(32))))
    .mod(bn(FIELD_SIZE.toString()))
    .toArray("be", 32);

async function main() {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const queue = keys.stateNullifierQueue;

  const alice = FIXED_PAYER;
  const bob = Keypair.generate();
  const connection = new Connection("http://localhost:8899", "confirmed");
  const aliceBalance = 1e9;
  const lamportsToSend = 5e8;

  const sig = await connection.requestAirdrop(FIXED_PAYER.publicKey, 2e9);
  await confirmTx(connection, sig);

  const in_utxos: UtxoWithBlinding[] = [
    {
      owner: alice.publicKey,
      lamports: aliceBalance / 2,
      blinding: rndMockedBlinding(),
      data: null,
    },
    {
      owner: alice.publicKey,
      lamports: aliceBalance / 2,
      blinding: rndMockedBlinding(),
      data: null,
    },
  ];

  const out_utxos: Utxo[] = [
    {
      owner: bob.publicKey,
      lamports: lamportsToSend,
      data: null,
    },
    {
      owner: alice.publicKey,
      lamports: aliceBalance - lamportsToSend,
      data: null,
    },
  ];

  const proof_mock: MockProof = {
    a: Array.from({ length: 32 }, () => 0),
    b: Array.from({ length: 64 }, () => 0),
    c: Array.from({ length: 32 }, () => 0),
  };

  const ix = await createExecuteCompressedInstruction(
    alice.publicKey,
    in_utxos,
    out_utxos,
    [merkleTree, merkleTree],
    [queue, queue],
    [merkleTree, merkleTree],
    [0, 0], // mock root indices
    proof_mock // mock zkp
  );
  const ixs = [ix];

  /// Build and send Solana tx
  const { blockhash } = await connection.getLatestBlockhash();

  const messageV0 = new TransactionMessage({
    payerKey: alice.publicKey,
    recentBlockhash: blockhash,
    instructions: ixs,
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  tx.sign([alice]);
  await sendAndConfirmTx(connection, tx);

  console.log(
    `\x1b[32mTransferred ${lamportsToSend} lamports from Alice to Bob\x1b[0m`
  );
  console.log("\x1b[34mInput mock-utxos (consumed):\x1b[0m");
  console.log(
    "\x1b[33mNOTE: the indexer will only be able to find output utxos, since we're mocking inputs in this example.\x1b[0m"
  );
  printUtxos(in_utxos);
  console.log("\x1b[34mOutput utxos (created):\x1b[0m");
  printOutUtxos(out_utxos);
}

main().catch((err) => {
  console.error(err);
});

const printUtxos = (utxos: UtxoWithBlinding[]) => {
  // nicely print the fields of each utxo into a table console
  // Display UTXOs in a structured table format for better readability
  const formattedUtxos = utxos.map((utxo) => ({
    ...utxo,
    owner: utxo.owner.toBase58(),
    lamports: utxo.lamports.toString(),
  }));
  console.table(formattedUtxos, ["owner", "lamports", "data", "blinding"]);
};

const printOutUtxos = (utxos: Utxo[]) => {
  // nicely print the fields of each utxo into a table console
  // Display UTXOs in a structured table format for better readability
  const formattedUtxos = utxos.map((utxo) => ({
    ...utxo,
    owner: utxo.owner.toBase58(),
    lamports: utxo.lamports.toString(),
  }));
  console.table(formattedUtxos, ["owner", "lamports", "data"]);
};
