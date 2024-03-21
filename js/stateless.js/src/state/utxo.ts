import { PublicKey } from '@solana/web3.js';
import { createTlvDataElement } from './utxo-data';
import { BN254, bn, createBN254 } from './BN254';
import { BN } from '@coral-xyz/anchor';
import { TlvDataElement_IdlType, Tlv_IdlType } from './types';

/** Describe the generic utxo details applicable to every utxo. */
export type Utxo = {
    /** Public key of program or user that owns the utxo */
    owner: PublicKey;
    /** Optional lamports attached to the utxo */
    lamports: BN;
    /** Optional data attached to the utxo */
    data: Tlv_IdlType | null;
    /**
     * TODO: Implement address functionality Optional unique account ID that is
     * persistent across transactions.
     */
    address: PublicKey | null;
};

/** Context for utxos inserted into a state Merkle tree */
export type MerkleContext = {
    /** State Merkle tree */
    merkleTree: PublicKey;
    /** the state nullfier queue belonging to merkleTree */
    nullifierQueue: PublicKey;
    /** Poseidon hash of the utxo preimage. Is a leaf in state merkle tree  */
    hash: BN254;
    /** 'hash' position within the Merkle tree */
    leafIndex: number | BN;
};

export type MerkleUpdateContext = {
    /** Slot that the utxo was appended at */
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

/** Utxo with Merkle tree context */
export type UtxoWithMerkleContext = Utxo & MerkleContext;

/** Utxo with Merkle proof and context */
export type UtxoWithMerkleProof = Utxo & MerkleContextWithMerkleProof;

/** Utxo object factory */
export const createUtxo = (
    owner: PublicKey,
    lamports: number | BN,
    data: Tlv_IdlType | null = null,
    address?: PublicKey,
): Utxo => ({
    owner,
    lamports: bn(lamports),
    data,
    address: address ?? null,
});

/** UtxoWithMerkleContext object factory */
export const createUtxoWithMerkleContext = (
    owner: PublicKey,
    lamports: number | BN,
    data: Tlv_IdlType | null = null,
    merkleContext: MerkleContext,
    address?: PublicKey,
): UtxoWithMerkleContext => ({
    owner,
    lamports: bn(lamports),
    data,
    address: address ?? null,
    ...merkleContext,
});

/** Add Merkle tree context to a utxo */
export const addMerkleContextToUtxo = (
    utxo: Utxo,
    hash: BN254,
    merkleTree: PublicKey,
    leafIndex: number | BN,
    nullifierQueue: PublicKey,
): UtxoWithMerkleContext => ({
    ...utxo,
    leafIndex,
    hash,
    merkleTree,
    nullifierQueue,
});

/** Append a merkle proof to a utxo */
export const addMerkleProofToUtxo = (
    utxo: UtxoWithMerkleContext,
    merkleProof: BN254[],
    recentRootIndex: number,
): UtxoWithMerkleProof => ({
    ...utxo,
    merkleProof,
    rootIndex: recentRootIndex,
});

// TODO: move to a separate file
/** Filter utxos with compressed lamports. Excludes PDAs and token accounts */
export function getCompressedSolUtxos(utxos: Utxo[]): Utxo[] {
    return utxos.filter(
        utxo => new BN(utxo.lamports).gt(new BN(0)) && utxo.data === null,
    );
}
/** Converts into UtxoWithMerkleContext[] type */
export function coerceIntoUtxoWithMerkleContext(
    utxos: (UtxoWithMerkleContext | UtxoWithMerkleProof)[],
): UtxoWithMerkleContext[] {
    return utxos.map((utxo): UtxoWithMerkleContext => {
        if ('merkleProof' in utxo && 'rootIndex' in utxo) {
            const { merkleProof, rootIndex, ...rest } = utxo;
            return rest;
        }
        return utxo;
    });
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    const mockTlvDataElement = (): TlvDataElement_IdlType =>
        createTlvDataElement(
            [1, 2, 3],
            new PublicKey(new Uint8Array([1, 2, 3])),
            new Uint8Array([1, 2, 3]),
            [1, 2, 3],
        );

    const mockTlv = (): Tlv_IdlType => ({
        tlvElements: [mockTlvDataElement()],
    });

    describe('getCompressedSolUtxos function', () => {
        it('should return utxos with compressed lamports excluding those with data', () => {
            const randomPubKeys = [
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
            ];
            const utxos = [
                createUtxo(randomPubKeys[0], bn(0), mockTlv()), // has data, should be excluded
                createUtxo(randomPubKeys[1], bn(1)), // valid
                createUtxo(randomPubKeys[2], bn(2)), // valid
                createUtxo(randomPubKeys[3], bn(0)), // zero lamports, should be excluded
            ];
            const solutxos = getCompressedSolUtxos(utxos);
            expect(solutxos).toEqual([
                createUtxo(randomPubKeys[1], bn(1)),
                createUtxo(randomPubKeys[2], bn(2)),
            ]);
        });

        it('should return an empty array when all utxos have data or zero lamports', () => {
            const randomPubKeys = [PublicKey.unique(), PublicKey.unique()];
            const utxos = [
                createUtxo(randomPubKeys[0], bn(0), mockTlv()), // has data
                createUtxo(randomPubKeys[1], bn(0)), // zero lamports
            ];
            const solutxos = getCompressedSolUtxos(utxos);
            expect(solutxos).toEqual([]);
        });
    });

    describe('coerceIntoUtxoWithMerkleContext function', () => {
        it('should return utxos with merkle context, excluding merkleProof and rootIndex', () => {
            const randomPubKeys = [
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
            ];
            const asBN254 = [
                createBN254(1),
                createBN254(2),
                createBN254(3),
                createBN254(4),
                createBN254(5),
                createBN254(6),
                createBN254(7),
                createBN254(8),
                createBN254(9),
            ];

            const utxoWithCtx0 = addMerkleContextToUtxo(
                createUtxo(randomPubKeys[2], bn(1)),
                bn(0),
                randomPubKeys[3],
                0,
                randomPubKeys[4],
            );

            const utxoWithCtx = addMerkleContextToUtxo(
                createUtxo(randomPubKeys[5], bn(2)),
                bn(1),
                randomPubKeys[6],
                1,
                randomPubKeys[7],
            );

            const utxos = [
                utxoWithCtx0,
                addMerkleProofToUtxo(utxoWithCtx, [asBN254[8]], 0),
            ];
            expect(coerceIntoUtxoWithMerkleContext(utxos)).toEqual([
                utxoWithCtx0,
                utxoWithCtx, // shouldn't have merkleProof and rootIndex
            ]);
        });

        it('should correctly handle an empty array input', () => {
            const utxos: (UtxoWithMerkleContext | UtxoWithMerkleProof)[] = [];
            const result = coerceIntoUtxoWithMerkleContext(utxos);
            expect(result).toEqual([]);
        });

        it('should return the same array if no utxos have merkleProof and rootIndex', () => {
            const randomPubKeys = [PublicKey.unique(), PublicKey.unique()];

            const utxoWithCtx = [
                addMerkleContextToUtxo(
                    createUtxo(randomPubKeys[0], bn(1)),
                    bn(0),
                    randomPubKeys[1],
                    0,
                    PublicKey.unique(),
                ),
            ];

            const result = coerceIntoUtxoWithMerkleContext(utxoWithCtx);
            expect(result).toEqual(utxoWithCtx);
        });
    });
}
