import { PublicKey } from "@solana/web3.js";
import { bigint254 } from "./bigint254";
import { LightSystemProgram } from "../programs/compressed-pda";
import { Buffer } from "buffer";

/** Describe the generic details applicable to every data block */
export type TlvDataElement = {
  discriminator: Uint8Array;
  /** Public key of the ownerProgram of the data block */
  owner: PublicKey;
  /** Variable-length data */
  data: Uint8Array;
  /** Poseidon hash of data */
  dataHash: Uint8Array; // Consider using bigint254
};

/** Describe the generic utxo details applicable to every utxo. */
export type Utxo = {
  /** Public key of program or user that owns the utxo */
  owner: PublicKey;
  /** Optional lamports attached to the utxo */
  lamports: bigint;
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
  /** State Merkle tree ID */
  merkletreeId: bigint;
  /** 'hash' position within the Merkle tree */
  leafIndex: bigint;
  /** Recent valid 'hash' proof path, expiring after n slots */
  merkleProof?: string[];
};

/** Utxo with Merkle tree context */
export type UtxoWithMerkleContext = Utxo & MerkleContext;

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

/** Adds Merkle tree context to a utxo */
export const addMerkleContextToUtxo = (
  utxo: Utxo,
  hash: bigint254,
  merkletreeId: bigint,
  leafIndex: bigint,
  merkleProof?: string[]
): UtxoWithMerkleContext => ({
  ...utxo,
  leafIndex,
  hash,
  merkletreeId,
  merkleProof,
});

/** Appends a merkle proof to a utxo */
export const addMerkleProofToUtxo = (
  utxo: UtxoWithMerkleContext,
  proof: string[]
): UtxoWithMerkleContext => ({
  ...utxo,
  merkleProof: proof,
});

/** Factory for TLV data elements */
export const createTlvDataElement = (
  discriminator: Uint8Array,
  owner: PublicKey,
  data: Uint8Array,
  dataHash: Uint8Array
): TlvDataElement => ({
  discriminator,
  owner,
  data,
  dataHash,
});

/** Filter utxos with native compressed lamports, excluding PDAs and token accounts */
export function getCompressedSolUtxos(utxos: Utxo[]): Utxo[] {
  return utxos.filter(
    (utxo) => utxo.lamports > BigInt(0) && utxo.data.length === 0
  );
}

const { coder } = LightSystemProgram.program;

/** Decode TLV data elements from a buffer */
export function decodeUtxoData(buffer: Buffer): TlvDataElement[] {
  return coder.types.decode("Tlv", buffer);
}

/** Encode TLV data elements into a buffer */
export function encodeUtxoData(data: TlvDataElement[]): Buffer {
  return coder.types.encode("Tlv", data);
}
