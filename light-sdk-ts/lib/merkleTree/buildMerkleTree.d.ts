import { Connection, PublicKey } from "@solana/web3.js";
import { MerkleTree } from "./merkleTree";
export declare const buildMerkleTree: ({
  connection,
  config,
  merkleTreePubkey,
  poseidonHash,
}: {
  connection: Connection;
  config: any;
  merkleTreePubkey: PublicKey;
  poseidonHash: any;
}) => Promise<MerkleTree>;
