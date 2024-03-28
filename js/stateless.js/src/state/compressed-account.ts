import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { CompressedAccount, CompressedAccountData } from './types';
import { BN254, bn, createBN254 } from './BN254';

export type CompressedAccountWithMerkleContext = CompressedAccount &
    MerkleContext;

/**
 * Context for compressed account inserted into a state Merkle tree
 * */
export type MerkleContext = {
    /** State Merkle tree */
    merkleTree: PublicKey;
    /** The state nullfier queue belonging to merkleTree */
    nullifierQueue: PublicKey;
    /** Poseidon hash of the utxo preimage. Is a leaf in state merkle tree  */
    hash: number[]; // TODO: BN254;
    /** 'hash' position within the Merkle tree */
    leafIndex: number;
};

export type MerkleUpdateContext = {
    /** Slot that the compressed account was appended at */
    slotCreated: number;
    /** Sequence */
    seq: number;
};

export type MerkleContextWithMerkleProof = MerkleContext & {
    /** Recent valid 'hash' proof path, expires after n slots */
    merkleProof: BN254[];
    /** Index of state root the merkleproof is valid for, expires after n slots */
    rootIndex: number;
};

export const createCompressedAccount = (
    owner: PublicKey,
    lamports?: BN,
    data?: CompressedAccountData,
    address?: PublicKey,
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
    address?: PublicKey,
): CompressedAccountWithMerkleContext => ({
    ...createCompressedAccount(owner, lamports, data, address),
    ...merkleContext,
});

export const createMerkleContext = (
    merkleTree: PublicKey,
    nullifierQueue: PublicKey,
    hash: number[], // TODO: BN254,
    leafIndex: number,
): MerkleContext => ({
    merkleTree,
    nullifierQueue,
    hash,
    leafIndex,
});

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    describe('createCompressedAccount function', () => {
        it('should create a compressed account with default values', () => {
            const owner = PublicKey.unique();
            const account = createCompressedAccount(owner);
            expect(account).toEqual({
                owner,
                lamports: bn(0),
                address: null,
                data: null,
            });
        });

        it('should create a compressed account with provided values', () => {
            const owner = PublicKey.unique();
            const lamports = bn(100);
            const data = {
                discriminator: [0],
                data: Buffer.from(new Uint8Array([1, 2, 3])),
                dataHash: [0],
            };
            const address = PublicKey.unique();
            const account = createCompressedAccount(
                owner,
                lamports,
                data,
                address,
            );
            expect(account).toEqual({
                owner,
                lamports,
                address,
                data,
            });
        });
    });

    describe('createCompressedAccountWithMerkleContext function', () => {
        it('should create a compressed account with merkle context', () => {
            const owner = PublicKey.unique();
            const merkleTree = PublicKey.unique();
            const nullifierQueue = PublicKey.unique();
            const hash = new Array(32).fill(1);
            const leafIndex = 0;
            const merkleContext = createMerkleContext(
                merkleTree,
                nullifierQueue,
                hash,
                leafIndex,
            );
            const accountWithMerkleContext =
                createCompressedAccountWithMerkleContext(merkleContext, owner);
            expect(accountWithMerkleContext).toEqual({
                owner,
                lamports: bn(0),
                address: null,
                data: null,
                merkleTree,
                nullifierQueue,
                hash,
                leafIndex,
            });
        });
    });

    describe('createMerkleContext function', () => {
        it('should create a merkle context', () => {
            const merkleTree = PublicKey.unique();
            const nullifierQueue = PublicKey.unique();
            const hash = new Array(32).fill(1);

            const leafIndex = 0;
            const merkleContext = createMerkleContext(
                merkleTree,
                nullifierQueue,
                hash,
                leafIndex,
            );
            expect(merkleContext).toEqual({
                merkleTree,
                nullifierQueue,
                hash,
                leafIndex,
            });
        });
    });
}
