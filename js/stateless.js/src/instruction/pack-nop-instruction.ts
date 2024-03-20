// TODO: refactor to be more coherent with pack-instruction. tho this will do
// for the test.
import {
  PublicKey,
  AccountMeta,
  TransactionInstruction,
} from '@solana/web3.js';
import { BorshCoder } from '@coral-xyz/anchor';

import {
  CompressedProof_IdlType,
  InUtxoTuple_IdlType,
  OutUtxoTuple_IdlType,
  Utxo,
  Utxo_IdlType,
} from '../state';

import { defaultStaticAccountsStruct } from '../constants';
import { LightSystemProgram } from '../programs/compressed-pda';

export async function createExecuteCompressedInstruction(
  payer: PublicKey,
  inUtxos: Utxo_IdlType[],
  outUtxos: Utxo[],
  inUtxoMerkleTreePubkeys: PublicKey[],
  nullifierArrayPubkeys: PublicKey[],
  outUtxoMerkleTreePubkeys: PublicKey[],
  rootIndices: number[],
  proof: CompressedProof_IdlType,
): Promise<TransactionInstruction> {
  const remainingAccounts = new Map<PublicKey, number>();
  const _inUtxos: InUtxoTuple_IdlType[] = [];
  inUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, remainingAccounts.size);
    }
    _inUtxos.push({
      inUtxo: inUtxos[i],
      indexMtAccount: remainingAccounts.get(mt)!,
      indexNullifierArrayAccount: 0, // TODO: dynamic!
    });
  });
  let len = remainingAccounts.size;
  nullifierArrayPubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, len + i);
    }
    _inUtxos[i].indexNullifierArrayAccount = remainingAccounts.get(mt)!;
  });
  len = remainingAccounts.size;
  const outputStateMerkleTreeAccountIndices: number[] = [];
  outUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, len + i);
    }
    outputStateMerkleTreeAccountIndices.push(remainingAccounts.get(mt)!);
  });

  const rawInputs = {
    proof,
    inputRootIndices: [...rootIndices],
    relayFee: null,
    inputCompressedAccountWithMerkleContext: _inUtxos,
    outputCompressedAccounts: outUtxos,
    outputStateMerkleTreeAccountIndices,
  };

  const staticAccounts = { ...defaultStaticAccountsStruct(), signer: payer };

  const accCoder = new BorshCoder(LightSystemProgram.program.idl);
  // remove disc
  const data = (
    await accCoder.accounts.encode('instructionDataTransfer', rawInputs)
  ).subarray(8);

  // const refEncodedData = [ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0,
  //
  //   0, 0, 0, 0, 0, 1, 0, 0, 0, 227, 130, 162, 184, 215, 227, 81, 211, 134,
  //   73, 118, 71, 219, 163, 243, 41, 118, 21, 155, 87, 11, 53, 153, 130, 178,
  //   126, 151, 86, 225, 36, 251, 130, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  //   1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 227, 130, 162, 184, 215, 227, 81, 211, 134,
  //   73, 118, 71, 219, 163, 243, 41, 118, 21, 155, 87, 11, 53, 153, 130, 178,
  //   126, 151, 86, 225, 36, 251, 130, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  // ];

  // // assert data is equal to refEncodedData if (data.length !==
  // refEncodedData.length) { throw new Error("data length mismatch");
  // }
  // for (let i = 0; i < data.length; i++) { if (data[i] !== refEncodedData[i])
  //   { throw new Error(`data mismatch at index ${i}`);
  //   }
  // }

  const remainingAccountMetas = Array.from(remainingAccounts.entries())
    .sort((a, b) => a[1] - b[1])
    .map(
      ([account]): AccountMeta => ({
        pubkey: account,
        isWritable: true, // TODO: check if inputmerkletrees should write
        isSigner: false,
      }),
    );

  const instruction = await LightSystemProgram.program.methods
    .executeCompressedTransaction(data)
    .accounts({
      ...staticAccounts,
      invokingProgram: LightSystemProgram.programId,
    })
    .remainingAccounts(remainingAccountMetas)
    .instruction();

  return instruction;
}
