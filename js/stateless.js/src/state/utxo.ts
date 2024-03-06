import { PublicKey } from "@solana/web3.js";
import { TlvDataElement, createTlvDataElement } from "./utxo-data";
import { bigint254, bn, createBigint254 } from "./bigint254";
import { BN } from "@coral-xyz/anchor";

/// TODO: implement PublicKey254 type based on bigint254
/// figure which fmt to use in indexer and on client side: since solana's regluar 'PublicKey' expects padding to 32.
export type PublicKey254 = PublicKey;

/** Describe the generic utxo details applicable to every utxo. */
export type Utxo = {
  /** Public key of program or user that owns the utxo */
  owner: PublicKey;
  /** Optional lamports attached to the utxo */
  lamports: number | BN;
  /** Optional data attached to the utxo */
  data: TlvDataElement[] | null;
  /**
   * TODO: Implement address functionality
   * Optional unique account ID that is persistent across transactions.
   */
  address?: PublicKey;
};

/** Context for utxos inserted into a state Merkle tree */
export type MerkleContext = {
  /** Poseidon hash of the utxo preimage  */
  hash: bigint254;
  /** State Merkle tree */
  merkleTree: PublicKey;
  /** 'hash' position within the Merkle tree */
  leafIndex: number;
  /** the state nullfier queue belonging to merkleTree */
  stateNullifierQueue: PublicKey;
};

export type MerkleUpdateContext = {
  /** Slot that the utxo was appended at */
  slotUpdated: number;
  /** Sequence */
  seq: number;
};

export type MerkleContextWithMerkleProof = MerkleContext & {
  /** Recent valid 'hash' proof path, expires after n slots */
  merkleProof: PublicKey254[];
  /** Index of state root the merkleproof is valid for, expires after n slots  */
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
  data: TlvDataElement[] = [],
  address?: PublicKey,
  merkleContext?: MerkleContext
): Utxo | UtxoWithMerkleContext => ({
  owner,
  lamports,
  data,
  address,
  ...merkleContext,
});

/** Add Merkle tree context to a utxo */
export const addMerkleContextToUtxo = (
  utxo: Utxo,
  hash: bigint254,
  merkleTree: PublicKey,
  leafIndex: number,
  stateNullifierQueue: PublicKey
  // merkleProof?: PublicKey254[] // TODO evaluate whether to add as optional
): UtxoWithMerkleContext => ({
  ...utxo,
  leafIndex,
  hash,
  merkleTree,
  stateNullifierQueue,
});

/** Append a merkle proof to a utxo */
export const addMerkleProofToUtxo = (
  utxo: UtxoWithMerkleContext,
  merkleProof: PublicKey254[],
  recentRootIndex: number
): UtxoWithMerkleProof => ({
  ...utxo,
  merkleProof,
  rootIndex: recentRootIndex,
});

// TODO: move to a separate file
/** Filter utxos with compressed lamports. Excludes PDAs and token accounts */
export function getCompressedSolUtxos(utxos: Utxo[]): Utxo[] {
  return utxos.filter(
    (utxo) => new BN(utxo.lamports) > new BN(0) && utxo.data?.length === 0
  );
}

/** Converts into UtxoWithMerkleContext[] type */
export function coerceIntoUtxoWithMerkleContext(
  utxos: (UtxoWithMerkleContext | UtxoWithMerkleProof)[]
): UtxoWithMerkleContext[] {
  return utxos.map((utxo): UtxoWithMerkleContext => {
    if ("merkleProof" in utxo && "rootIndex" in utxo) {
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

  const mockTlvDataElement = (): TlvDataElement =>
    createTlvDataElement(
      new Uint8Array([1, 2, 3]),
      new PublicKey(new Uint8Array([1, 2, 3])),
      new Uint8Array([1, 2, 3]),
      createBigint254(1)
    );

  describe("getCompressedSolUtxos function", () => {
    it("should return utxos with compressed lamports excluding those with data", () => {
      const randomPubKeys = [
        PublicKey.unique(),
        PublicKey.unique(),
        PublicKey.unique(),
        PublicKey.unique(),
      ];
      const utxos = [
        createUtxo(randomPubKeys[0], bn(0), [mockTlvDataElement()]), // has data, should be excluded
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

    it("should return an empty array when all utxos have data or zero lamports", () => {
      const randomPubKeys = [PublicKey.unique(), PublicKey.unique()];
      const utxos = [
        createUtxo(randomPubKeys[0], bn(0), [mockTlvDataElement()]), // has data
        createUtxo(randomPubKeys[1], bn(0)), // zero lamports
      ];
      const solutxos = getCompressedSolUtxos(utxos);
      expect(solutxos).toEqual([]);
    });
  });

  describe("coerceIntoUtxoWithMerkleContext function", () => {
    it("should return utxos with merkle context, excluding merkleProof and rootIndex", () => {
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

      const utxoWithCtx0 = addMerkleContextToUtxo(
        createUtxo(randomPubKeys[2], bn(1)),
        BigInt(0),
        randomPubKeys[3],
        0,
        randomPubKeys[4]
      );

      const utxoWithCtx = addMerkleContextToUtxo(
        createUtxo(randomPubKeys[5], bn(2)),
        BigInt(1),
        randomPubKeys[6],
        1,
        randomPubKeys[7]
      );

      const utxos = [
        utxoWithCtx0,
        addMerkleProofToUtxo(utxoWithCtx, [randomPubKeys[8]], 0),
      ];
      expect(coerceIntoUtxoWithMerkleContext(utxos)).toEqual([
        utxoWithCtx0,
        utxoWithCtx, // shouldn't have merkleProof and rootIndex
      ]);
    });

    it("should correctly handle an empty array input", () => {
      const utxos: (UtxoWithMerkleContext | UtxoWithMerkleProof)[] = [];
      const result = coerceIntoUtxoWithMerkleContext(utxos);
      expect(result).toEqual([]);
    });

    it("should return the same array if no utxos have merkleProof and rootIndex", () => {
      const randomPubKeys = [PublicKey.unique(), PublicKey.unique()];

      const utxoWithCtx = [
        addMerkleContextToUtxo(
          createUtxo(randomPubKeys[0], bn(1)),
          BigInt(0),
          randomPubKeys[1],
          0,
          PublicKey.unique()
        ),
      ];

      const result = coerceIntoUtxoWithMerkleContext(utxoWithCtx);
      expect(result).toEqual(utxoWithCtx);
    });
  });
}
