import { PublicKey } from "@solana/web3.js";
import { TlvDataElement } from "./utxo-data";

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
  hash: PublicKey254;
  /** State Merkle tree */
  merkleTree: PublicKey;
  /** 'hash' position within the Merkle tree */
  leafIndex: number;
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
};

/** Utxo with Merkle tree context */
export type UtxoWithMerkleContext = Utxo & MerkleContext;

/** Utxo with Merkle proof and context */
export type UtxoWithMerkleProof = Utxo & MerkleContextWithMerkleProof;

/** Utxo object factory */
export const createUtxo = (
  owner: PublicKey,
  lamports: bigint,
  data: TlvDataElement[],
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
  hash: PublicKey254,
  merkleTree: PublicKey,
  leafIndex: number
  // merkleProof?: PublicKey254[] // TODO evaluate whether to add as optional
): UtxoWithMerkleContext => ({
  ...utxo,
  leafIndex,
  hash,
  merkleTree,
});

/** Append a merkle proof to a utxo */
export const addMerkleProofToUtxo = (
  utxo: UtxoWithMerkleContext,
  proof: PublicKey254[]
): UtxoWithMerkleProof => ({
  ...utxo,
  merkleProof: proof,
});

// TODO: move to a separate file
/** Filter utxos with compressed lamports. Excludes PDAs and token accounts */
export function getCompressedSolUtxos(utxos: Utxo[]): Utxo[] {
  return utxos.filter(
    (utxo) => utxo.lamports > BigInt(0) && utxo.data.length === 0
  );
}
