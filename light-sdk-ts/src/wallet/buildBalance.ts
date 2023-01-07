import {Utxo} from "../utxo";
import * as anchor from "@project-serum/anchor";
import { MerkleTreeProgram } from "../../idls/merkle_tree_program";
import { assert, expect } from "chai";
const token = require('@solana/spl-token')
import {Connection, PublicKey, Keypair} from "@solana/web3.js";
import { MINT } from "../constants";

export async function getUninsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection
  // merkleTreePubkey
}: {
  merkleTreeProgram: MerkleTreeProgram,
  merkleTreeIndex: any,
  connection: Connection

}) {
  var leave_accounts: Array<{
    pubkey: PublicKey
    account: Account<Buffer>
  }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
  console.log("Total nr of accounts. ", leave_accounts.length);

  let filteredLeaves = leave_accounts
  .filter((pda) => {
    return pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber()
  }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

  return filteredLeaves.map((pda) => {
      return { isSigner: false, isWritable: false, pubkey: pda.publicKey};
  })
}

export async function getUnspentUtxo(leavesPdas, provider: anchor.Provider, 
    encryptionKeypair, KEYPAIR, FEE_ASSET,mint,
    POSEIDON, merkleTreeProgram: MerkleTreeProgram
  ) {
  let decryptedUtxo1
  for (var i = 0; i < leavesPdas.length; i++) {
    console.log("iter ", i);
    try {
       // decrypt first leaves account and build utxo
      decryptedUtxo1 = Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(0,71))), new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(71, 71+24))), encryptionKeypair.PublicKey, encryptionKeypair, KEYPAIR, [FEE_ASSET,mint], POSEIDON, 0)[1];
      console.log("decryptedUtxo1 ", decryptedUtxo1);
      
      let nullifier = decryptedUtxo1.getNullifier();
      console.log("decryptedUtxo1", decryptedUtxo1);

      let nullifierPubkey = (await PublicKey.findProgramAddress(
          [new anchor.BN(nullifier.toString()).toBuffer(), anchor.utils.bytes.utf8.encode("nf")],
          merkleTreeProgram.programId))[0]
      let accountInfo = await provider.connection.getAccountInfo(nullifierPubkey);
      console.log("accountInfo ", accountInfo);
      console.log("decryptedUtxo1.amounts[1].toString()  ", decryptedUtxo1.amounts[1].toString() );
      console.log("decryptedUtxo1.amounts[0].toString()  ", decryptedUtxo1.amounts[0].toString() );

      if (accountInfo == null && decryptedUtxo1.amounts[1].toString() != "0" && decryptedUtxo1.amounts[0].toString() != "0") {
        console.log("found unspent leaf");
        return decryptedUtxo1;
      } else if (i == leavesPdas.length - 1) {
        throw "no unspent leaf found";
      }
    } catch (error) {
      console.log(error);
      
    }
   

  }

}

export async function getInsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection
  // merkleTreePubkey
}: {
  merkleTreeProgram: MerkleTreeProgram,
  connection: Connection,
  merkleTreeIndex: any
})/*: Promise<{ pubkey: PublicKey; account: Account<Buffer>; }[]>*/ {
  var leave_accounts: Array<{
    pubkey: PublicKey
    account: Account<Buffer>
  }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
  console.log("Total nr of accounts. ", leave_accounts.length);

  let filteredLeaves = leave_accounts
  .filter((pda) => {
    return pda.account.leftLeafIndex.toNumber() < merkleTreeIndex.toNumber()
  }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

  return filteredLeaves;
}
