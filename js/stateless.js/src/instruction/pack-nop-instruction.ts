// TODO: refactor to be more coherent with pack-instruction. tho this will do for the test.
import {
  PublicKey,
  AccountMeta,
  TransactionInstruction,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

import { Utxo } from "../state";

import { defaultStaticAccounts } from "../constants";
import { LightSystemProgram } from "../programs/compressed-pda";
import { ValidityProof } from "./validity-proof";

export type UtxoWithBlinding = Utxo & {
  blinding: number[]; // 32 bytes, leafIndex
  //   lamports: BN; // no BN!
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

export async function createExecuteCompressedInstruction(
  payer: PublicKey,
  inUtxos: Utxo[],
  outUtxos: Utxo[],
  inUtxoMerkleTreePubkeys: PublicKey[],
  nullifierArrayPubkeys: PublicKey[],
  outUtxoMerkleTreePubkeys: PublicKey[],
  rootIndices: number[],
  proof: ValidityProof
): Promise<TransactionInstruction> {
  let remainingAccounts = new Map<PublicKey, number>();
  let _inUtxos: InUtxoTuple[] = [];
  inUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccounts.has(mt)) {
      remainingAccounts.set(mt, remainingAccounts.size);
    }
    _inUtxos.push({
      inUtxo: { ...inUtxos[i], blinding: new Array(32).fill(0) }, // think we need to attach leafIndex as blinding here!
      indexMtAccount: remainingAccounts.get(mt)!,
      indexNullifierArrayAccount: 0,
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
    rpcFee: new BN(0),
    inUtxos: _inUtxos.map((utxo) => ({
      ...utxo,
      inUtxo: {
        ...utxo.inUtxo,
        lamports: new BN(utxo.inUtxo.lamports.toString()),
      },
    })),
    outUtxos: _outUtxos.map((utxo) => ({
      ...utxo,
      outUtxo: {
        ...utxo.outUtxo,
        lamports: new BN(utxo.outUtxo.lamports.toString()),
      },
    })),
    rootIndices: [...rootIndices],
    proof,
    // proof: { // see idl!
    //   a: proof.proofA,
    //   b: proof.proof_b,
    //   c: proof.proof_c,
    // },
  };

  let staticAccounts = [payer, ...defaultStaticAccounts()];

  const data = await LightSystemProgram.program.coder.accounts.encode(
    "instructionDataTransfer",
    rawInputs
  );

  const staticAccountMetas = staticAccounts.map(
    (account): AccountMeta => ({
      pubkey: account,
      isWritable: false,
      isSigner: true, // signers
    })
  );
  const remainingAccountMetas = Array.from(remainingAccounts.entries())
    .sort((a, b) => a[1] - b[1])
    .map(
      ([account]): AccountMeta => ({
        pubkey: account,
        isWritable: true, // TODO: check if inputmerkletrees should write
        isSigner: false,
      })
    );

  let instruction = new TransactionInstruction({
    programId: LightSystemProgram.programId,
    keys: [...staticAccountMetas, ...remainingAccountMetas],
    data,
  });
  return instruction;
}
