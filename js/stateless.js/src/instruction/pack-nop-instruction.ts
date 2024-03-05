// TODO: refactor to be more coherent with pack-instruction. tho this will do for the test.
import {
  PublicKey,
  AccountMeta,
  TransactionInstruction,
} from "@solana/web3.js";
import { BN, BorshCoder } from "@coral-xyz/anchor";

import { Utxo } from "../state";

import {
  defaultStaticAccounts,
  defaultStaticAccountsStruct,
} from "../constants";
import { LightSystemProgram } from "../programs/compressed-pda";

/// Temporary fix for congruence with the current anchor IDL while we're switching to use leafindex+mt as part of the UtxoWithMerkleContext type.
export type UtxoWithBlinding = Utxo & {
  blinding: number[]; // 32 bytes, leafIndex
};

export type InUtxoTuple = {
  inUtxo: UtxoWithBlinding; // think we need to attach leafIndex as blinding here!
  indexMtAccount: number;
  indexNullifierArrayAccount: number;
};

export type OutUtxoTuple = {
  outUtxo: Utxo;
  indexMtAccount: number;
};

export type MockProof = {
  a: number[];
  b: number[];
  c: number[];
};

export async function createExecuteCompressedInstruction(
  payer: PublicKey,
  inUtxos: Utxo[],
  outUtxos: Utxo[],
  inUtxoMerkleTreePubkeys: PublicKey[],
  nullifierArrayPubkeys: PublicKey[],
  outUtxoMerkleTreePubkeys: PublicKey[],
  rootIndices: number[],
  proof: MockProof
): Promise<TransactionInstruction> {
  let remainingAccounts = new Map<PublicKey, number>();
  let _inUtxos: InUtxoTuple[] = [];
  inUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, remainingAccounts.size);
    }
    _inUtxos.push({
      //@ts-ignore
      inUtxo: inUtxos[i], // { ...inUtxos[i], blinding: new Array(32).fill(0) }, // think we need to attach leafIndex as blinding here!
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
  let _outUtxos: OutUtxoTuple[] = [];
  outUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, len + i);
    }
    _outUtxos.push({
      outUtxo: outUtxos[i],
      indexMtAccount: remainingAccounts.get(mt)!,
    });
  });

  // hack!
  let rawInputs = {
    lowElementIndices: new Array(inUtxos.length).fill(0),
    relayFee: null,
    inUtxos: _inUtxos.map((utxo) => ({
      ...utxo,
      inUtxo: {
        ...utxo.inUtxo,
        lamports: new BN(utxo.inUtxo.lamports.toString()), // Number(utxo.inUtxo.lamports),
      },
    })),
    outUtxos: _outUtxos.map((utxo) => ({
      ...utxo,
      outUtxo: {
        ...utxo.outUtxo,
        lamports: new BN(utxo.outUtxo.lamports.toString()), //Number(utxo.outUtxo.lamports),
      },
    })),
    rootIndices: [...rootIndices],
    proof,
  };

  const staticAccounts = { ...defaultStaticAccountsStruct(), signer: payer };

  const accCoder = new BorshCoder(LightSystemProgram.program.idl);

  // remove disc
  const data = (
    await accCoder.accounts.encode("instructionDataTransfer", rawInputs)
  ).subarray(8);

  // const refEncodedData = [
  //   1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 227, 130,
  //   162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41, 118, 21,
  //   155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130, 1, 1, 1, 1,
  //   1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  //   1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 227, 130, 162, 184,
  //   215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41, 118, 21, 155, 87,
  //   11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130, 0, 0, 0, 0, 0, 0, 0,
  //   0, 0, 0,
  // ];

  // // assert data is equal to refEncodedData
  // if (data.length !== refEncodedData.length) {
  //   throw new Error("data length mismatch");
  // }
  // for (let i = 0; i < data.length; i++) {
  //   if (data[i] !== refEncodedData[i]) {
  //     throw new Error(`data mismatch at index ${i}`);
  //   }
  // }

  const remainingAccountMetas = Array.from(remainingAccounts.entries())
    .sort((a, b) => a[1] - b[1])
    .map(
      ([account]): AccountMeta => ({
        pubkey: account,
        isWritable: true, // TODO: check if inputmerkletrees should write
        isSigner: false,
      })
    );

  const instruction = await LightSystemProgram.program.methods
    .executeCompressedTransaction(data)
    .accounts({ ...staticAccounts })
    .remainingAccounts(remainingAccountMetas)
    .instruction();

  return instruction;
}
