import { Connection, PublicKey } from '@solana/web3.js';
import { MerkleTree } from './merkleTree';
export declare const buildMerkleTree: ({ connection, config, merkleTreePubkey, merkleTreeProgram, poseidonHash }: {
    connection: Connection;
    config: any;
    merkleTreePubkey: PublicKey;
    merkleTreeProgram: any;
    poseidonHash: any;
}) => Promise<MerkleTree>;
