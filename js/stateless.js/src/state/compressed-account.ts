import { PublicKey } from '@solana/web3.js';
import {
    CompressedAccount,
    CompressedAccountData,
    StateTreeInfo,
} from './types';
import BN from 'bn.js';
import { BN254, bn } from './BN254';

export type CompressedAccountWithMerkleContext = CompressedAccount &
    MerkleContext & {
        readOnly: boolean;
    };

/**
 * @deprecated use {@link MerkleContext} instead.
 *
 * Legacy MerkleContext
 */
export type MerkleContextLegacy = {
    /**
     * State tree
     */
    merkleTree: PublicKey;
    /**
     * Nullifier queue
     */
    nullifierQueue: PublicKey;
    /**
     * Poseidon hash of the account. Stored as leaf in state tree
     */
    hash: number[];
    /**
     * Position of `hash` in the State tree
     */
    leafIndex: number;
};

/**
 * Context for compressed account stored in a state tree
 */
export type MerkleContext = {
    /**
     * Tree info
     */
    treeInfo: StateTreeInfo;
    /**
     * Poseidon hash of the account. Stored as leaf in state tree
     */
    hash: BN;
    /**
     * Position of `hash` in the State tree
     */
    leafIndex: number;
    /**
     * Whether the account can be proven by index or by merkle proof
     */
    proveByIndex: boolean;
};

/**
 * MerkleContext with merkle proof
 */
export type MerkleContextWithMerkleProof = MerkleContext & {
    /**
     * Recent valid 'hash' proof path, expires after n slots
     */
    merkleProof: BN254[];
    /**
     * Index of state root the merkleproof is valid for, expires after n slots
     */
    rootIndex: number;
    /**
     * Current root
     */
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
    treeInfo: StateTreeInfo,
    hash: BN254,
    leafIndex: number,
    proveByIndex: boolean = false,
): MerkleContext => ({
    treeInfo,
    hash,
    leafIndex,
    proveByIndex,
});
