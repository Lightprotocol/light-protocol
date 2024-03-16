import {
  CompressedProof_IdlType,
  Utxo,
  Utxo_IdlType,
  bn,
} from '../../src/state';
import { describe, it } from 'vitest';
import { byteArrayToKeypair } from '../../src/test-utils/init-accounts';
import {
  Connection,
  TransactionMessage,
  VersionedTransaction,
  Keypair,
} from '@solana/web3.js';
import { createExecuteCompressedInstruction } from '../../src/instruction/pack-nop-instruction';
import { FIELD_SIZE, defaultTestStateTreeAccounts } from '../../src/constants';
import { confirmTx, sendAndConfirmTx } from '../../src/test-utils';
import crypto from 'crypto';

/// static testing key. don't use in prod.
const FIXED_PAYER = byteArrayToKeypair([
  122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
  158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
  227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
  118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
]);

// creates mock blinding < bn254 field size.
// random to generate unique utxos. currently we don't enforce root/index/ checks.
// This will be replaced by blinding(merkletreePubkey,leafIndex)
const rndMockedBlinding = () =>
  bn(Array.from(crypto.getRandomValues(new Uint8Array(32))))
    .mod(bn(FIELD_SIZE.toString()))
    .toArray('be', 32);

/// emit events in a loop
const rounds = 1;

describe('Emit events', () => {
  it('should execute a compressed lamport transfer and emit events correctly', async () => {
    for (let i = 0; i < rounds; i++) {
      /// Get testing keys. tree and queue are auto-initialized with the test-validator env.
      const keys = defaultTestStateTreeAccounts();
      const merkleTree = keys.merkleTree;

      const alice = FIXED_PAYER;
      const bob = Keypair.generate();
      const connection = new Connection('http://localhost:8899', 'confirmed');
      const lamportsToSend = 0;

      const sig = await connection.requestAirdrop(FIXED_PAYER.publicKey, 2e9);
      await confirmTx(connection, sig);

      /// Define input state (current state that the tx consumes)
      // Note:
      // We don't compress SOL yet, therefore cannot spend utxos with value yet.
      // TODO: add one run with with inputUtxo where lamports: 0
      const in_utxos: Utxo_IdlType[] = [];

      /// Define output state (new state that the tx creates)
      /// In this example, Alice sends 42e7 lamports to Bob and keeps the rest back.
      /// These utxos get emitted and are indexable.
      /// Note: we're running lowest level utxo assembly here for the sake of the example.
      /// Users will get helpers for this.
      /// Think: ```const ix = LightSystemProgram.transfer(fromBalance, to, lamports) ...```
      const out_utxos: Utxo[] = [
        {
          owner: bob.publicKey,
          lamports: bn(lamportsToSend),
          data: null,
          address: null,
        },
        {
          owner: alice.publicKey,
          lamports: bn(0),
          data: null,
          address: null,
        },
      ];

      const proof_mock: CompressedProof_IdlType = {
        a: Array.from({ length: 32 }, () => 0),
        b: Array.from({ length: 64 }, () => 0),
        c: Array.from({ length: 32 }, () => 0),
      };

      /// Packs the utxos into ixdata and encodes the data with the relevant system keys.
      /// This is the "not-optimized" version. devs will use the more efficient packInstruction() instead.
      /// The packing doesnt have any impact on the emitted events / indexing experience though.
      const ix = await createExecuteCompressedInstruction(
        alice.publicKey,
        in_utxos,
        out_utxos,
        [],
        [],
        [merkleTree, merkleTree],
        [], // mock root indices
        proof_mock, // mock zkp
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

      const txId = await sendAndConfirmTx(connection, tx);

      /// Prints
      console.log(
        `\n\n\n\n tx: https://explorer.solana.com/tx/${txId}?cluster=custom`,
      );
      console.log(
        `\x1b[32mTransferred ${lamportsToSend} lamports from Alice (${alice.publicKey.toBase58()}) to Bob (${bob.publicKey.toBase58()})\x1b[0m`,
      );
      console.log('\x1b[34mInput mock-utxos (consumed):\x1b[0m');
      console.log(
        "\x1b[33mNOTE: the indexer will only be able to find output utxos since we're mocking inputs in this example.\x1b[0m",
      );
      printUtxos(in_utxos);
      console.log('\n\x1b[34mOutput utxos (created):\x1b[0m');
      printOutUtxos(out_utxos);
    }
  });
});

const printUtxos = (utxos: Utxo_IdlType[]) => {
  const formattedUtxos = utxos.map((utxo) => ({
    ...utxo,
    owner: utxo.owner.toBase58(),
    lamports: utxo.lamports.toString(),
  }));
  console.table(formattedUtxos, ['owner', 'lamports', 'data']);
  utxos.forEach((utxo, index) => {
    console.log(
      `Blinding for input utxo ${index + 1}: ${utxo.blinding.toString()}`,
    );
  });
};

const printOutUtxos = (utxos: Utxo[]) => {
  const formattedUtxos = utxos.map((utxo) => ({
    ...utxo,
    owner: utxo.owner.toBase58(),
    lamports: utxo.lamports.toString(),
  }));
  console.table(formattedUtxos, ['owner', 'lamports', 'data']);
};
