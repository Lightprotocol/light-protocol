import { Connection, PublicKey } from "@solana/web3.js";
export declare function checkMerkleTreeUpdateStateCreated({
  connection,
  merkleTreeUpdateState,
  relayer,
  MerkleTree,
  leavesPdas,
  current_instruction_index,
  merkleTreeProgram,
}: {
  connection: any;
  merkleTreeUpdateState: any;
  relayer: any;
  MerkleTree: any;
  leavesPdas: any;
  current_instruction_index: any;
  merkleTreeProgram: any;
}): Promise<void>;
export declare function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
  leavesPdas,
  merkleTree,
  merkle_tree_pubkey,
  merkleTreeProgram,
}: {
  connection: any;
  merkleTreeUpdateState: any;
  merkleTreeAccountPrior: any;
  numberOfLeaves: any;
  leavesPdas: any;
  merkleTree: any;
  merkle_tree_pubkey: any;
  merkleTreeProgram: any;
}): Promise<void>;
export declare function checkRentExemption({
  connection,
  account,
}: {
  connection: any;
  account: any;
}): Promise<void>;
export declare function checkNfInserted(
  pubkeys: PublicKey[],
  connection: Connection
): Promise<void>;
