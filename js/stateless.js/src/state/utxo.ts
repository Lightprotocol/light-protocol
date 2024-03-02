import { PublicKey } from "@solana/web3.js";
import { TlvDataElement, createTlvDataElement } from "./utxo-data";
import { bigint254, createBigint254 } from "./bigint254";

/// TODO: implement PublicKey254 type based on bigint254
/// figure which fmt to use in indexer and on client side: since solana's regluar 'PublicKey' expects padding to 32.
export type PublicKey254 = PublicKey;

/** Describe the generic utxo details applicable to every utxo. */
export type Utxo = {
  /** Public key of program or user that owns the utxo */
  owner: PublicKey;
  /** Optional lamports attached to the utxo */
  lamports: number | bigint;
  /** Optional data attached to the utxo */
  data: TlvDataElement[];
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
  lamports: number | bigint,
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
  return utxos.filter((utxo) => utxo.lamports > BigInt(0) && !utxo.data);
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

/// akin to in conversion.ts, add vitest best practice unit test cases for the above functions
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
    it("should return utxos with compressed lamports", () => {
      const utxos = [
        createUtxo(new PublicKey("1"), BigInt(0), [mockTlvDataElement()]),
        createUtxo(new PublicKey("2"), BigInt(1)),
        createUtxo(new PublicKey("3"), BigInt(2)),
        createUtxo(new PublicKey("4"), BigInt(0)),
      ];
      expect(getCompressedSolUtxos(utxos)).toEqual([
        createUtxo(new PublicKey("2"), BigInt(1)),
        createUtxo(new PublicKey("3"), BigInt(2)),
      ]);
    });
  });

  describe("coerceIntoUtxoWithMerkleContext function", () => {
    it("should return utxos with merkle context", () => {
      const utxos = [
        addMerkleContextToUtxo(
          createUtxo(new PublicKey("2"), BigInt(1)),
          BigInt(0),
          new PublicKey("3"),
          0,
          new PublicKey("4")
        ),
        addMerkleProofToUtxo(
          addMerkleContextToUtxo(
            createUtxo(new PublicKey("5"), BigInt(2)),
            BigInt(1),
            new PublicKey("6"),
            1,
            new PublicKey("7")
          ),
          [new PublicKey("8")],
          0
        ),
      ];
      expect(coerceIntoUtxoWithMerkleContext(utxos)).toEqual([
        createUtxo(new PublicKey("1"), BigInt(0), [mockTlvDataElement()]),
        createUtxo(new PublicKey("2"), BigInt(1)),
      ]);
    });
  });
}
