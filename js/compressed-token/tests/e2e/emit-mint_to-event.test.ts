import { beforeAll, describe, it } from 'vitest';

import {
  Connection,
  TransactionMessage,
  VersionedTransaction,
  Keypair,
} from '@solana/web3.js';

import {
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE } from '../../src/constants';
import { CompressedTokenProgram } from '../../src/program';

/// static testing key. don't use in prod.
const FIXED_PAYER = byteArrayToKeypair([
  122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
  158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
  227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
  118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
]);

/// This is for a randomly generated mint:
/// GDvagojL2e9B7Eh7CHwHjQwcJAAtiMpbvCvtzDTCpogP using FIXED_MINT lets you
/// create multiple rounds of mint_to events for the same mint
const FIXED_MINT = byteArrayToKeypair([
  133, 115, 36, 85, 197, 163, 96, 25, 135, 202, 109, 119, 13, 73, 54, 129, 75,
  247, 52, 249, 6, 95, 72, 142, 66, 100, 61, 132, 76, 118, 160, 83, 226, 46,
  219, 140, 17, 189, 22, 168, 53, 214, 179, 106, 62, 218, 202, 149, 113, 147,
  83, 16, 247, 15, 109, 251, 238, 102, 186, 48, 251, 212, 159, 44,
]);

/// emit mint_to events in a loop
const mintToRounds = 1;

describe('Emit events for mint and mint_to', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const payer = FIXED_PAYER;
  const bob = Keypair.generate();
  const connection = new Connection('http://localhost:8899', 'confirmed');

  const mint = FIXED_MINT;
  const mintDecimals = 2;

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 3e9);
    await confirmTx(connection, sig);
  });

  /// Emits mint_to events on-chain Adjust mintToRounds to emit more mint_to
  /// events in a loop
  it('should mint_to bob', async () => {
    for (let i = 0; i < mintToRounds; i++) {
      const ix = await CompressedTokenProgram.mintTo({
        feePayer: payer.publicKey,
        mint: mint.publicKey,
        authority: payer.publicKey,
        amount: 100 * mintDecimals,
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
        } compressed tokens (mint: ${mint.publicKey.toBase58()}) to bob \n txId: ${txId}`,
      );
      /// TODO(swen): assert output and print output utxos after implementing proper beet serde
    }
  });
});
