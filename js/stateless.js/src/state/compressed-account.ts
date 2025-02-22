import BN from 'bn.js';
import { PublicKey } from '@solana/web3.js';
import { CompressedAccount, CompressedAccountData } from './types';
import { BN254, bn } from './BN254';

export type CompressedAccountWithMerkleContext = CompressedAccount &
    MerkleContext & {
        readOnly: boolean;
    };

/**
 * V1: State Merkle trees; V2: Batched Merkle Tree. Default: V2 for outputs. V2
 * transactions store outputs in the `queue` account instead of the `merkleTree`
 * account.
 */
export enum MerkleContextVersion {
    V1 = 1,
    V2 = 2,
}

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
    version: MerkleContextVersion;
    /** Whether to prove by index or by validity proof */
    proveByIndex: boolean;
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
    version: MerkleContextVersion,
    proveByIndex: boolean,
): MerkleContext => ({
    merkleTree,
    queue,
    hash,
    leafIndex,
    version,
    proveByIndex,
});
