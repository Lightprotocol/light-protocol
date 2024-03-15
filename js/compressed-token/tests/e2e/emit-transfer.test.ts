import { beforeAll, describe, it } from 'vitest';

import {
  Connection,
  TransactionMessage,
  VersionedTransaction,
  Keypair,
  ComputeBudgetProgram,
} from '@solana/web3.js';

import {
  CompressedProof_IdlType,
  FIELD_SIZE,
  TlvDataElement_IdlType,
  Utxo_IdlType,
  bn,
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../src/program';
import { createTransferInstruction } from '../../src/instructions/transfer';
import { BN } from '@coral-xyz/anchor';
import {
  TokenTlvData_IdlType,
  TokenTransferOutUtxo_IdlType,
} from '../../src/types';

/// static testing key. don't use in prod.
const FIXED_PAYER = byteArrayToKeypair([
  122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
  158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
  227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
  118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
]);

/// This is for a randomly generated mint:
/// GDvagojL2e9B7Eh7CHwHjQwcJAAtiMpbvCvtzDTCpogP using FIXED_MINT lets you
/// create multiple rounds of transfer events for the same mint
const FIXED_MINT = byteArrayToKeypair([
  133, 115, 36, 85, 197, 163, 96, 25, 135, 202, 109, 119, 13, 73, 54, 129, 75,
  247, 52, 249, 6, 95, 72, 142, 66, 100, 61, 132, 76, 118, 160, 83, 226, 46,
  219, 140, 17, 189, 22, 168, 53, 214, 179, 106, 62, 218, 202, 149, 113, 147,
  83, 16, 247, 15, 109, 251, 238, 102, 186, 48, 251, 212, 159, 44,
]);

/// This is a randomly generated keypair for bob
/// FeH3NxoYJFJpHkTrmR8wyk63rST8mBhpLrgtMgH19Ay6
/// using this keypair lets you mint_to bob repeatedly,
/// which then can further be used to transfer to other accounts
const FIXED_BOB = byteArrayToKeypair([
  23, 72, 199, 170, 152, 40, 30, 187, 91, 132, 88, 170, 94, 32, 89, 164, 164,
  38, 123, 3, 79, 17, 23, 83, 112, 91, 160, 140, 116, 9, 99, 38, 217, 144, 62,
  153, 200, 117, 213, 6, 62, 39, 186, 56, 34, 149, 58, 188, 99, 182, 87, 74, 84,
  182, 157, 45, 133, 253, 230, 193, 176, 160, 72, 249,
]);

// creates mock blinding < bn254 field size.
// random to generate unique utxos. currently we don't enforce root/index/ checks.
// This will be replaced by blinding(merkletreePubkey,leafIndex)
const rndMockedBlinding = () =>
  bn(Array.from(crypto.getRandomValues(new Uint8Array(32))))
    .mod(bn(FIELD_SIZE.toString()))
    .toArray('be', 32);

/// emit mint_to events in a loop
const transferRounds = 1;

describe('Emit events for transfer', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const queue = keys.nullifierQueue;
  const payer = FIXED_PAYER;
  const bob = FIXED_BOB;
  const charlie = Keypair.generate(); // rand
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const transferAmount = 7;

  const mintDecimals = 1e2;

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 3e9);
    await confirmTx(connection, sig);
  });

  /// Emits compressed token transfer events on-chain Adjust transferRounds to emit more transfer
  /// events in a loop
  it('should transfer bob -> charlie', async () => {
    for (let i = 0; i < transferRounds; i++) {
      const tlv: TokenTlvData_IdlType = {
        mint: FIXED_MINT.publicKey,
        owner: bob.publicKey,
        amount: bn(1000 + transferAmount * mintDecimals), // 1000 is the mocked input balance - transferAmount
        delegate: null,
        state: 0x01, //'Initialized',
        isNative: null,
        delegatedAmount: bn(0),
      };

      const tlvData = CompressedTokenProgram.program.coder.types.encode(
        'TokenTlvData',
        tlv,
      );

      const tlvDataElement: TlvDataElement_IdlType = {
        discriminator: Array(8).fill(2),
        owner: bob.publicKey,
        data: Uint8Array.from(tlvData),
        dataHash: Array(32).fill(0), // mock!
      };

      const inUtxo: Utxo_IdlType = {
        owner: bob.publicKey,
        blinding: rndMockedBlinding(),
        lamports: new BN(0),
        data: { tlvElements: [tlvDataElement] },
        address: null,
      };

      const changeUtxo: TokenTransferOutUtxo_IdlType = {
        amount: bn(1000),
        owner: bob.publicKey,
        lamports: null,
        index_mt_account: 0,
      };

      let charlieOutUtxo: TokenTransferOutUtxo_IdlType = {
        amount: bn(transferAmount * mintDecimals),
        owner: charlie.publicKey,
        lamports: null,
        index_mt_account: 0,
      };

      const proof_mock: CompressedProof_IdlType = {
        a: Array.from({ length: 32 }, () => 0),
        b: Array.from({ length: 64 }, () => 0),
        c: Array.from({ length: 32 }, () => 0),
      };

      const ix = await createTransferInstruction(
        payer.publicKey,
        bob.publicKey,
        [merkleTree],
        [queue],
        [merkleTree, merkleTree],
        [inUtxo],
        [charlieOutUtxo, changeUtxo],
        [0], // input state root indices
        proof_mock,
      );

      /// Build and send Solana tx
      const { blockhash } = await connection.getLatestBlockhash();
      const messageV0 = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: blockhash,
        instructions: [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ix,
        ],
      }).compileToV0Message();
      const tx = new VersionedTransaction(messageV0);
      tx.sign([payer, bob]);

      const txId = await sendAndConfirmTx(connection, tx);

      console.log(
        `bob (${bob.publicKey.toBase58()}) transferred ${transferAmount * mintDecimals} tokens (mint: ${FIXED_MINT.publicKey.toBase58()}) to charlie (${charlie.publicKey.toBase58()}) \n txId: ${txId}`,
      );
      /// TODO(swen): assert output and print output utxos after implementing proper beet serde
    }
  });
});
