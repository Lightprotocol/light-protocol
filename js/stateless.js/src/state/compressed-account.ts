import { PublicKey } from '@solana/web3.js';
import {
    CompressedAccountData,
    CompressedAccountLegacy,
    PackedMerkleContextLegacy,
    TreeInfo,
} from './types';
import BN from 'bn.js';
import { BN254 } from './BN254';

/**
 * @deprecated use {@link CompressedAccount} instead
 */
export type CompressedAccountWithMerkleContext = CompressedAccount &
    MerkleContext & {
        readOnly: boolean;
    };

/**
 * @deprecated use {@link CompressedAccount} instead
 */
export type CompressedAccountWithMerkleContextLegacy = CompressedAccount &
    MerkleContext;

/**
 * Compressed account + metadata about the state tree in which the account is
 * stored.
 */
export type CompressedAccount = {
    /**
     * Public key of program or user owning the account.
     */
    owner: PublicKey;
    /**
     * Lamports attached to the account.
     */
    lamports: BN;
    /**
     * Optional unique account ID that is persistent across transactions.
     */
    address: number[] | null;
    /**
     * Optional data attached to the account.
     */
    data: CompressedAccountData | null;
} & MerkleContext & {
        /**
         * Read only.
         */
        readOnly: boolean;
    };

/**
 * @deprecated use {@link MerkleContext} instead.
 *
 * Legacy MerkleContext.
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
    treeInfo: TreeInfo;
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

/**
 * Packed compressed account and state tree info.
 */
export type PackedStateTreeInfo = {
    /**
     * Recent valid root index.
     */
    rootIndex: number;
    /**
     * Whether the account can be proven by index or by merkle proof
     */
    proveByIndex: boolean;
    /**
     * Index of the merkle tree in which the account is stored.
     */
    merkleTreePubkeyIndex: number;
    /**
     * Index of the queue in which the account is stored.
     */
    queuePubkeyIndex: number;
    /**
     * Index of the leaf in the state tree.
     */
    leafIndex: number;
};

/**
 * Packed tree info for a new program-derived address (PDA).
 */
export type PackedAddressTreeInfo = {
    /**
     * Index of the merkle tree in which the account is stored.
     */
    addressMerkleTreePubkeyIndex: number;
    /**
     * Index of the queue in which the account is stored.
     */
    addressQueuePubkeyIndex: number;
    /**
     * Recent valid root index.
     */
    rootIndex: number;
};

/**
 * Compressed account meta in instruction.
 *
 */
export type CompressedAccountMeta = {
    /**
     * Packed Tree info.
     */
    treeInfo: PackedStateTreeInfo;
    /**
     * Address.
     */
    address: number[] | null;
    /**
     * Lamports.
     */
    lamports: BN | null;
    /**
     * index of state tree in which the new account state is stored.
     */
    outputStateTreeIndex: number;
};

/**
 * Create an output compressed account meta for a new account.
 * Client-side only.
 */
export const createCompressedAccountMeta = (
    treeInfo: PackedStateTreeInfo,
    outputStateTreeIndex: number,
    address?: number[],
    lamports?: BN,
): CompressedAccountMeta => ({
    treeInfo,
    outputStateTreeIndex,
    address: address ?? null,
    lamports: lamports ?? null,
});

/**
 * @deprecated Use {@link PackedStateTreeInfo} instead.
 * Packed compressed account with merkle context.
 */
export interface PackedCompressedAccountWithMerkleContext {
    /**
     * Compressed account.
     */
    compressedAccount: CompressedAccountLegacy;
    /**
     * Merkle context.
     */
    merkleContext: PackedMerkleContextLegacy;
    /**
     * Root index.
     */
    rootIndex: number;
    /**
     * Read only.
     */
    readOnly: boolean;
}

/**
 * @deprecated use {@link createCompressedAccountMeta} instead.
 */
export const createCompressedAccountLegacy = (
    owner: PublicKey,
    lamports?: BN,
    data?: CompressedAccountData,
    address?: number[],
): CompressedAccountLegacy => ({
    owner,
    lamports: lamports ?? new BN(0),
    address: address ?? null,
    data: data ?? null,
});
/**
 * @deprecated.
 */
export const createCompressedAccountWithMerkleContextLegacy = (
    merkleContext: MerkleContext,
    owner: PublicKey,
    lamports?: BN,
    data?: CompressedAccountData,
    address?: number[],
): CompressedAccountWithMerkleContext => ({
    ...merkleContext,
    owner,
    lamports: lamports ?? new BN(0),
    address: address ?? null,
    data: data ?? null,
    readOnly: false,
});

/**
 * @deprecated use {@link createCompressedAccountMeta} instead.
 */
export const createMerkleContextLegacy = (
    treeInfo: TreeInfo,
    hash: BN254,
    leafIndex: number,
    proveByIndex: boolean = false,
): MerkleContext => ({
    treeInfo,
    hash,
    leafIndex,
    proveByIndex,
});
