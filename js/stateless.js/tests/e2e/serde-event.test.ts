import { describe, it, expect, assert, beforeAll } from 'vitest';
import { createUtxo } from '../../src/state';
import {
  PAYER_KEYPAIR,
  byteArrayToKeypair,
} from '../../src/test-utils/init-accounts';

import {
  PublicKey,
  Connection,
  TransactionMessage,
  VersionedTransaction,
  TransactionConfirmationStrategy,
  Keypair,
} from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
  MockProof,
  createExecuteCompressedInstruction,
  UtxoWithBlinding,
} from '../../src/instruction/pack-nop-instruction';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import {
  PublicTransactionIndexerEventBeet,
  confirmTx,
  getMockRpc,
  sendAndConfirmTx,
} from '../../src/test-utils';

const referencePubEvent = [
  1, 0, 0, 0, 0, 68, 77, 125, 32, 76, 128, 61, 180, 1, 207, 69, 44, 121, 118,
  153, 17, 179, 183, 115, 34, 163, 127, 102, 214, 1, 87, 175, 177, 95, 49, 65,
  69, 1, 0, 0, 0, 27, 0, 0, 0, 13, 133, 147, 43, 77, 70, 41, 23, 221, 17, 183,
  126, 184, 224, 252, 59, 136, 249, 73, 153, 2, 243, 20, 245, 141, 26, 118, 134,
  12, 50, 143, 83, 0, 0, 0, 4, 5, 21, 223, 180, 186, 115, 40, 85, 110, 47, 13,
  133, 62, 117, 110, 179, 47, 22, 139, 170, 227, 89, 96, 71, 85, 171, 80, 112,
  235, 106, 96, 146, 0, 0, 0, 2, 28, 242, 224, 51, 52, 132, 122, 72, 33, 151,
  108, 112, 115, 173, 66, 174, 134, 5, 131, 23, 170, 85, 142, 18, 8, 200, 132,
  17, 52, 90, 92, 31, 0, 0, 0, 1, 25, 190, 28, 192, 134, 162, 153, 89, 22, 122,
  83, 55, 107, 179, 131, 205, 170, 145, 26, 246, 21, 41, 254, 212, 108, 117,
  113, 190, 164, 32, 200, 229, 0, 0, 128, 0, 35, 87, 224, 101, 23, 35, 242, 1,
  196, 41, 213, 122, 79, 77, 31, 98, 230, 152, 147, 115, 12, 54, 220, 220, 245,
  197, 176, 49, 98, 137, 254, 241, 0, 0, 64, 0, 43, 110, 57, 208, 197, 25, 203,
  181, 229, 132, 5, 185, 90, 129, 102, 228, 132, 17, 39, 231, 122, 10, 50, 40,
  236, 188, 84, 70, 24, 43, 3, 229, 0, 0, 32, 0, 5, 123, 134, 81, 118, 89, 231,
  23, 99, 175, 206, 66, 233, 65, 61, 255, 168, 126, 166, 125, 217, 70, 87, 95,
  43, 229, 59, 47, 84, 177, 151, 67, 0, 0, 16, 0, 28, 6, 29, 63, 3, 146, 209,
  104, 71, 229, 222, 174, 16, 120, 6, 177, 103, 26, 149, 202, 42, 136, 1, 219,
  185, 63, 75, 215, 115, 174, 83, 167, 0, 0, 8, 0, 47, 139, 119, 47, 31, 103,
  237, 50, 244, 13, 49, 43, 118, 192, 11, 228, 221, 144, 9, 253, 52, 84, 114,
  103, 62, 123, 243, 213, 53, 33, 140, 159, 0, 0, 4, 0, 35, 198, 44, 26, 77,
  172, 32, 136, 139, 38, 34, 187, 172, 15, 76, 180, 193, 62, 168, 112, 200, 9,
  49, 5, 23, 160, 211, 117, 229, 107, 203, 30, 0, 0, 2, 0, 38, 130, 238, 229,
  115, 42, 150, 62, 134, 215, 109, 0, 123, 195, 46, 29, 186, 186, 45, 82, 216,
  145, 34, 255, 34, 55, 76, 97, 244, 227, 67, 190, 0, 0, 1, 0, 14, 102, 41, 159,
  2, 30, 67, 48, 19, 153, 250, 106, 145, 175, 227, 115, 206, 124, 169, 89, 191,
  74, 79, 128, 241, 157, 122, 17, 211, 243, 1, 112, 0, 128, 0, 0, 0, 71, 128,
  223, 238, 43, 239, 90, 212, 83, 47, 83, 255, 68, 174, 192, 238, 244, 85, 114,
  75, 170, 41, 38, 195, 91, 130, 34, 32, 134, 121, 46, 0, 64, 0, 0, 3, 102, 143,
  27, 255, 151, 119, 194, 112, 41, 65, 251, 28, 32, 131, 200, 93, 77, 12, 210,
  181, 174, 191, 83, 165, 117, 251, 108, 30, 202, 144, 210, 0, 32, 0, 0, 6, 40,
  137, 56, 74, 9, 213, 160, 193, 214, 207, 248, 151, 211, 31, 59, 163, 61, 93,
  57, 139, 177, 43, 216, 104, 80, 111, 175, 229, 115, 89, 254, 0, 16, 0, 0, 24,
  186, 235, 195, 50, 179, 28, 127, 99, 88, 184, 15, 107, 105, 173, 199, 22, 191,
  7, 193, 245, 16, 34, 49, 115, 120, 1, 253, 221, 241, 152, 5, 0, 8, 0, 0, 35,
  69, 244, 31, 207, 178, 32, 207, 24, 202, 23, 214, 29, 186, 202, 98, 46, 70,
  132, 110, 165, 1, 9, 49, 53, 93, 64, 169, 208, 175, 188, 135, 0, 4, 0, 0, 18,
  223, 162, 206, 249, 106, 241, 60, 212, 121, 104, 168, 155, 210, 16, 121, 57,
  108, 245, 144, 123, 56, 181, 180, 148, 169, 139, 151, 39, 150, 192, 192, 0, 2,
  0, 0, 21, 186, 132, 83, 135, 123, 115, 181, 131, 193, 18, 6, 243, 103, 62,
  184, 48, 28, 81, 190, 194, 154, 13, 255, 123, 183, 83, 187, 111, 201, 215,
  162, 0, 1, 0, 0, 18, 51, 155, 103, 161, 192, 75, 16, 148, 30, 253, 115, 10,
  221, 152, 5, 243, 2, 176, 75, 236, 223, 244, 214, 29, 194, 73, 96, 253, 87,
  239, 217, 128, 0, 0, 0, 45, 15, 38, 130, 91, 79, 251, 170, 37, 168, 0, 57, 19,
  132, 28, 161, 0, 40, 241, 235, 75, 228, 101, 191, 134, 106, 100, 143, 218,
  244, 137, 235, 64, 0, 0, 0, 3, 46, 229, 105, 174, 107, 172, 58, 65, 194, 204,
  28, 93, 85, 31, 220, 93, 212, 144, 99, 213, 184, 231, 70, 105, 3, 240, 255,
  63, 180, 234, 232, 32, 0, 0, 0, 26, 231, 119, 234, 196, 81, 72, 151, 195, 29,
  253, 120, 234, 245, 56, 225, 62, 30, 6, 50, 200, 196, 171, 127, 36, 52, 72,
  23, 207, 227, 126, 223, 16, 0, 0, 0, 32, 85, 70, 30, 4, 79, 35, 28, 230, 203,
  219, 135, 218, 13, 30, 90, 127, 157, 139, 117, 134, 72, 184, 239, 226, 115,
  247, 214, 187, 189, 83, 2, 8, 0, 0, 0, 15, 166, 21, 72, 185, 212, 86, 207, 50,
  222, 141, 192, 161, 52, 147, 109, 39, 153, 239, 170, 189, 9, 29, 240, 129, 24,
  138, 229, 115, 182, 111, 111, 4, 0, 0, 0, 4, 125, 232, 70, 32, 89, 233, 84,
  150, 164, 188, 137, 208, 243, 252, 181, 3, 4, 16, 219, 107, 223, 99, 132, 15,
  157, 6, 166, 67, 91, 216, 114, 2, 0, 0, 0, 12, 201, 131, 15, 151, 188, 43, 53,
  252, 15, 168, 94, 102, 237, 173, 202, 57, 73, 85, 229, 169, 125, 15, 37, 103,
  250, 198, 61, 234, 95, 68, 227, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0,
];

const deserializeTransactionEvents = (data: number[]) => {
  // data = Buffer.from(Array.from(data).map((x: any) => Number(x)));

  try {
    const event = PublicTransactionIndexerEventBeet.struct.deserialize(
      Buffer.from(data),
    )[0];
    return event;
  } catch (e) {
    console.log('couldnt deserializing event', e);
    return null;
  }
};

describe('Serde test', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const queue = keys.stateNullifierQueue;
  const payer = byteArrayToKeypair([
    122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
    158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
    227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
    118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
  ]);
  console.log('payer', payer.publicKey.toBytes());
  const bob = Keypair.generate();
  const connection = new Connection('http://localhost:8899', 'confirmed');

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 2e9);
    await confirmTx(connection, sig);
  });

  it.only('should deserialize Public Tx Event correctly', async () => {
    console.log('len', referencePubEvent.length);

    // try deserialize manually
    deserializeTransactionEvents(referencePubEvent);
  });

  /// TODO: switch default tests to this after we resolve the deserialization bug
  it.skip('should match reference bytes for encoded inputs @test_execute_compressed_transactio (rust sdk)', async () => {
    const in_utxos: UtxoWithBlinding[] = [
      {
        owner: payer.publicKey,
        lamports: new BN(0),
        blinding: new Array(32).fill(1),
        data: null,
      },
    ];
    const out_utxos = [
      { owner: payer.publicKey, lamports: new BN(0), data: null },
    ];

    const proof_mock: MockProof = {
      a: Array.from({ length: 32 }, () => 0),
      b: Array.from({ length: 64 }, () => 0),
      c: Array.from({ length: 32 }, () => 0),
    };

    const ix = await createExecuteCompressedInstruction(
      payer.publicKey,
      in_utxos,
      out_utxos,
      [merkleTree],
      [queue],
      [merkleTree],
      [0],
      proof_mock,
    );
    const ixs = [ix];

    const { blockhash } = await connection.getLatestBlockhash();

    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);

    tx.sign([payer]);

    await sendAndConfirmTx(connection, tx);

    const mockRpc = getMockRpc(connection);
    const indexedEvents = await mockRpc.getIndexedEvents();

    assert.equal(indexedEvents.length, 1);
    assert.equal(indexedEvents[0].inUtxos.length, 1);
    assert.equal(indexedEvents[0].outUtxos.length, 1);
    assert.equal(indexedEvents[0].outUtxos[0].lamports, 0);
    assert.equal(
      indexedEvents[0].outUtxos[0].owner,
      payer.publicKey.toBase58(),
    );
    assert.equal(indexedEvents[0].outUtxos[0].data, null);
  });

  it.skip('should send .5 sol from alice to bob, with .5 change', async () => {
    const in_utxos: UtxoWithBlinding[] = [
      {
        owner: payer.publicKey,
        lamports: new BN(1e8),
        blinding: new Array(32).fill(1),
        data: null,
      },
      {
        owner: payer.publicKey,
        lamports: new BN(9e8),
        blinding: new Array(32).fill(1),
        data: null,
      },
    ];
    const out_utxos = [
      { owner: bob.publicKey, lamports: new BN(5e8), data: null },
      { owner: payer.publicKey, lamports: new BN(5e8), data: null },
    ];

    const proof_mock: MockProof = {
      a: Array.from({ length: 32 }, () => 0),
      b: Array.from({ length: 64 }, () => 0),
      c: Array.from({ length: 32 }, () => 0),
    };

    const ix = await createExecuteCompressedInstruction(
      payer.publicKey,
      in_utxos,
      out_utxos,
      [merkleTree, merkleTree],
      [queue, queue],
      [merkleTree, merkleTree],
      [0, 0],
      proof_mock,
    );
    const ixs = [ix];

    /// Build and send Solana tx
    const { blockhash } = await connection.getLatestBlockhash();

    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);

    tx.sign([payer]);
    await sendAndConfirmTx(connection, tx);

    /// Assert emitted events
    const mockRpc = getMockRpc(connection);
    const indexedEvents = await mockRpc.getIndexedEvents();

    assert.equal(indexedEvents.length, 2);

    assert.equal(indexedEvents[1].inUtxos.length, 2);
    assert.equal(indexedEvents[1].outUtxos.length, 2);
    assert.equal(indexedEvents[1].outUtxos[0].lamports, 5e8);
    assert.equal(indexedEvents[1].outUtxos[1].lamports, 5e8);
    assert.equal(indexedEvents[1].outUtxos[0].owner, bob.publicKey.toBase58());
    assert.equal(
      indexedEvents[1].outUtxos[1].owner,
      payer.publicKey.toBase58(),
    );
    assert.equal(indexedEvents[1].outUtxos[0].data, null);
    assert.equal(indexedEvents[1].outUtxos[1].data, null);
  });

  /// TODO: enable test after refactor for packInstruction() is complete
  it.skip('should build ix and send to chain successfully', async () => {
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
      createUtxo(recipient, 0),
      createUtxo(payer.publicKey, 0),
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
      mockProof,
    );

    const ixs = [ix];
    const connection = new Connection('http://localhost:8899', 'confirmed');

    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash();
    const balancePayer = await connection.getBalance(payer.publicKey);
    const balanceRecipient = await connection.getBalance(recipient);
    console.log('balance', balancePayer, balanceRecipient);

    const sig = await connection.requestAirdrop(payer.publicKey, 2e9);

    const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
      signature: sig,
      blockhash,
      lastValidBlockHeight,
    };
    console.log('confirming...', sig);
    await connection.confirmTransaction(
      transactionConfirmationStrategy,
      'confirmed',
    );
    console.log('sig', sig, 'payer', payer.publicKey.toBase58());
    const balancePayerAfterAirdrop = await connection.getBalance(
      payer.publicKey,
      'confirmed',
    );
    console.log('balancePayerAfterAirdrop', balancePayerAfterAirdrop);

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
        ].toBase58()} - Signer: ${tx.message.isAccountSigner(index)}`,
      );
    });
    tx.sign([payer]);

    console.log('tx', tx.signatures, '\n', tx.message.getAccountKeys());
    const txid = await connection.sendTransaction(tx);

    console.log(
      `https://explorer.solana.com/tx/${txid}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
    );
    expect(txid).toBeTruthy();
  });
});
