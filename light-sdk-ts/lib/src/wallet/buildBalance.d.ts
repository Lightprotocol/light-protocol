/// <reference types="node" />
import { Utxo } from "../utxo";
import * as anchor from "@project-serum/anchor";
import { MerkleTreeProgram } from "../../idls/merkle_tree_program";
import { Connection, PublicKey } from "@solana/web3.js";
export declare function getUninsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection,
}: {
  merkleTreeProgram: MerkleTreeProgram;
  merkleTreeIndex: any;
  connection: Connection;
}): Promise<
  {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: any;
  }[]
>;
export declare function getUnspentUtxo(
  leavesPdas: any,
  provider: anchor.Provider,
  encryptionKeypair: any,
  KEYPAIR: any,
  FEE_ASSET: any,
  mint: any,
  POSEIDON: any,
  merkleTreeProgram: MerkleTreeProgram,
): Promise<boolean | Utxo | null | undefined>;
export declare function getInsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection,
}: {
  merkleTreeProgram: MerkleTreeProgram;
  connection: Connection;
  merkleTreeIndex: any;
}): Promise<
  {
    pubkey: PublicKey;
    account: Account<Buffer>;
  }[]
>;
