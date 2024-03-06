import { describe, it, expect, assert, beforeAll } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE } from '../../src/constants';
import {
  Connection,
  Keypair,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import {
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
} from '@lightprotocol/stateless.js';

// new todo
// - test case 1 utxo with lamports in - 2 out utxos -> send money DONE
// - rename rpc fee to relay fee ~~
// - test changelog event nn
// - test rpc get utxo by owner THIS
// - clean up FIRST RUN DON
// - script that does one transaction with constanst as amounts so that these are easy to change
// - readme (start test validator, in separate tab run script that does a transaction)

// TODO add tests for
// - invalid tx signer
// - asserting emitted events..
// - repeat: sending fetching balance, sending more
// its not pretty but should work
describe('Compressed Token Program test', () => {
  // const keys = defaultTestStateTreeAccounts();
  // const merkleTree = keys.merkleTree;
  // const queue = keys.stateNullifierQueue;
  const payer = byteArrayToKeypair([
    122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
    158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
    227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
    118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
  ]);
  const bob = Keypair.generate();
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const randomMint = Keypair.generate();
  const mintDecimals = 2;

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 2e9);
    await confirmTx(connection, sig);
  });

  it('should create mint', async () => {
    const rentExemptBalance = SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE;

    const ixs = await CompressedTokenProgram.createMint({
      feePayer: payer.publicKey,
      mint: randomMint.publicKey,
      decimals: mintDecimals,
      authority: payer.publicKey,
      freezeAuthority: null,
      rentExemptBalance: rentExemptBalance,
    });

    /// Build and send Solana tx
    const { blockhash } = await connection.getLatestBlockhash();

    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    tx.sign([payer, randomMint]);

    const txId = await sendAndConfirmTx(connection, tx);

    console.log('created compressed Mint txId', txId);
  });

  it('should mint_to bob', async () => {
    const { merkleTree } = defaultTestStateTreeAccounts();

    const ix = await CompressedTokenProgram.mintTo({
      feePayer: payer.publicKey,
      mint: randomMint.publicKey,
      authority: payer.publicKey,
      amount: 1 * mintDecimals,
      toPubkey: bob.publicKey,
      merkleTree,
    });

    /// Build and send Solana tx
    const { blockhash } = await connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: [ix],
    }).compileToV0Message();
    const tx = new VersionedTransaction(messageV0);
    tx.sign([payer]);

    const txId = await sendAndConfirmTx(connection, tx);

    console.log(
      `minted ${
        1 * mintDecimals
      } tokens (mint: ${randomMint.publicKey.toBase58()}) to bob \n txId: ${txId}`,
    );
  });

  // TODO: add test for 'should batch mint'
  // TODO: add asserthelpers

  // it('should match reference bytes for encoded inputs @test_execute_compressed_transactio (rust sdk)', async () => {
  //   const in_utxos: UtxoWithBlinding[] = [
  //     {
  //       owner: payer.publicKey,
  //       lamports: new BN(0),
  //       blinding: new Array(32).fill(1),
  //       data: null,
  //     },
  //   ];
  //   const out_utxos = [
  //     { owner: payer.publicKey, lamports: new BN(0), data: null },
  //   ];

  //   const proof_mock: MockProof = {
  //     a: Array.from({ length: 32 }, () => 0),
  //     b: Array.from({ length: 64 }, () => 0),
  //     c: Array.from({ length: 32 }, () => 0),
  //   };

  //   const ix = await createExecuteCompressedInstruction(
  //     payer.publicKey,
  //     in_utxos,
  //     out_utxos,
  //     [merkleTree],
  //     [queue],
  //     [merkleTree],
  //     [0],
  //     proof_mock,
  //   );
  //   const ixs = [ix];

  //   const { blockhash } = await connection.getLatestBlockhash();

  //   const messageV0 = new TransactionMessage({
  //     payerKey: payer.publicKey,
  //     recentBlockhash: blockhash,
  //     instructions: ixs,
  //   }).compileToV0Message();

  //   const tx = new VersionedTransaction(messageV0);

  //   tx.sign([payer]);

  //   await sendAndConfirmTx(connection, tx);

  //   const mockRpc = getMockRpc(connection);
  //   const indexedEvents = await mockRpc.getIndexedEvents();

  //   assert.equal(indexedEvents.length, 1);
  //   assert.equal(indexedEvents[0].inUtxos.length, 1);
  //   assert.equal(indexedEvents[0].outUtxos.length, 1);
  //   assert.equal(indexedEvents[0].outUtxos[0].lamports, 0);
  //   assert.equal(
  //     indexedEvents[0].outUtxos[0].owner,
  //     payer.publicKey.toBase58(),
  //   );
  //   assert.equal(indexedEvents[0].outUtxos[0].data, null);
  // });
});
