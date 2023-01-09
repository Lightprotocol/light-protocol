import { Program } from '@coral-xyz/anchor';
import { Connection, PublicKey } from '@solana/web3.js';
import { MerkleTreeProgramIdl } from 'idls';
import { MerkleTree } from './merkleTree';
export declare const buildMerkleTree: ({ connection, config, merkleTreePubkey, merkleTreeProgram, poseidonHash }: {
    connection: Connection;
    config: any;
    merkleTreePubkey: PublicKey;
    merkleTreeProgram: Program<MerkleTreeProgramIdl>;
    poseidonHash: any;
}) => Promise<MerkleTree>;
