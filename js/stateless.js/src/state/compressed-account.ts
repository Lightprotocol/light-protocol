import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import {
    CompressedAccount,
    CompressedAccountData,
    OutputCompressedAccountWithPackedContext,
} from './types';
import { BN254, bn } from './BN254';
import { Buffer } from 'buffer';

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
    nullifierQueue: PublicKey;
    /** Poseidon hash of the utxo preimage. Is a leaf in state merkle tree  */
    hash: number[]; // TODO: BN254;
    /** 'hash' position within the Merkle tree */
    leafIndex: number;
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
            const address = Array.from(PublicKey.unique().toBytes());
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
                readOnly: false,
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
