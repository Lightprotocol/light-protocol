import BN from 'bn.js';
import { PublicKey } from '@solana/web3.js';
import { CompressedAccount, CompressedAccountData, TreeType } from './types';
import { BN254, bn } from './BN254';

export type CompressedAccountWithMerkleContext = CompressedAccount &
    MerkleContext & {
        readOnly: boolean;
    };

/**
 * Context for compressed account inserted into a state Merkle tree
 * */
export type MerkleContext = {
    /** State Merkle tree */
    merkleTree: PublicKey;
    /** The state nullfier queue belonging to merkleTree */
    queue: PublicKey;
    /** Poseidon hash of the utxo preimage. Is a leaf in state merkle tree  */
    hash: number[];
    /** 'hash' position within the Merkle tree */
    leafIndex: number;
    /** Version */
    treeType: TreeType;
    /** Whether to prove by index or by validity proof */
    proveByIndex: boolean;
};
/**
 * Context for compressed account inserted into a state Merkle tree V1
 * */
export type MerkleContextV1 = {
    /** State Merkle tree */
    merkleTree: PublicKey;
    /** The state nullfier queue belonging to merkleTree */
    queue: PublicKey;
    /** Poseidon hash of the utxo preimage. Is a leaf in state merkle tree  */
    hash: number[];
    /** 'hash' position within the Merkle tree */
    leafIndex: number;
};

export type MerkleContextWithMerkleProofV1 = MerkleContextV1 & {
    /** Recent valid 'hash' proof path, expires after n slots */
    merkleProof: BN254[];
    /** Index of state root the merkleproof is valid for, expires after n slots */
    rootIndex: number;
    /** Current root */
    root: BN254;
};

export type MerkleContextWithMerkleProof = MerkleContext & {
    /** Recent valid 'hash' proof path, expires after n slots */
    merkleProof: BN254[];
    /** Index of state root the merkleproof is valid for, expires after n slots */
    rootIndex: number;
    /** Current root */
    root: BN254;
};

export const createCompressedAccount = (
    owner: PublicKey,
    lamports?: BN,
    data?: CompressedAccountData,
    address?: number[],
): CompressedAccount => ({
    owner,
    lamports: lamports ?? bn(0),
    address: address ?? null,
    data: data ?? null,
});

export const createCompressedAccountWithMerkleContext = (
    merkleContext: MerkleContext,
    owner: PublicKey,
    lamports?: BN,
    data?: CompressedAccountData,
    address?: number[],
): CompressedAccountWithMerkleContext => ({
    ...createCompressedAccount(owner, lamports, data, address),
    ...merkleContext,
    readOnly: false,
});

export const createMerkleContext = (
    merkleTree: PublicKey,
    queue: PublicKey,
    hash: number[], // TODO: BN254,
    leafIndex: number,
    treeType: TreeType,
    proveByIndex: boolean,
): MerkleContext => ({
    merkleTree,
    queue,
    hash,
    leafIndex,
    treeType,
    proveByIndex,
});
