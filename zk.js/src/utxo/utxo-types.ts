import {BN254} from "utxo/bn254";
import {PublicKey} from "@solana/web3.js";
import {BN} from "@coral-xyz/anchor";

/** Public key of Poseidon-hashed keypair */
export type CompressionPublicKey = BN254;

/** Describes the generic utxo details applicable to every utxo. */
export type BaseUtxo = {
    /** Identifier and commitment to the utxo, is inserted as leaf into state tree */
    hash: BN254;
    /** Compression public key of the user or public key program owning the utxo */
    owner: CompressionPublicKey | PublicKey;
    /** Optional number of lamports and SPL amount assigned to the utxo */
    amounts: BN[];
    /** Optional native mint and SPL mint address respective to 'amounts' */
    assets: PublicKey[];
    /** Random value to force uniqueness of 'hash' */
    blinding: BN254;
    /** Optional type of utxo for custom circuits, defaults to 'native' */
    type: string;
    /** Default to '0' */
    version: string;
    /** Default to '0' */
    poolType: string;
    /** Indicator for whether the utxo is empty, for readability */
    isFillingUtxo: boolean;
    /** Default 'true'. Whether the inputs to 'hash' are public or not. Useful for confidential compute. */
    isPublic: boolean;
    /** Optional persistent id of the utxo. Used for compressed PDAs and non-fungible tokens */
    address?: BN254;
    /** Optional public key of program that owns the metadata */
    metadataOwner?: PublicKey;
    /**
     *	metadata which is immutable in normal transactions.
     *	metadata can be updated by the metadataOwner with a dedicated system psp.
     */
    metadata?: any; /// TODO: add metadata type
    // /** hash of metadata */
    // metadataHash?: string;
    /** hash of metadataHash and metadataOwner */
    metaHash?: BN254;
};

/** Utxo that had previously been inserted into a state Merkle tree */
export type Utxo = Omit<BaseUtxo, "owner"> & {
    /** Compression public key of the user that owns the utxo */
    owner: CompressionPublicKey;
    /** Hash that invalidates utxo once inserted into nullifier queue, if isPublic = true it defaults to: 'hash' */
    nullifier: BN254;
    /** Numerical identifier of the Merkle tree which the 'hash' is part of */
    merkletreeId: number;
    /** Proof path attached to the utxo. Can be reconstructed using event history */
    merkleProof: string[];
    /** Index of 'hash' as inserted into the Merkle tree. Max safe tree depth using number type would be **52, roughly 4.5 x 10^15 leaves */
    merkleTreeLeafIndex: number;
};

/** Utxo that is not inserted into the state tree yet. */
export type OutUtxo = Omit<BaseUtxo, "owner"> & {
    /** Compression public key of the user that owns the utxo */
    owner: CompressionPublicKey;
    /**
     * Optional public key of the ouput utxo owner once inserted into the state tree.
     * Only set if the utxo should be encrypted asymetrically.
     */
    encryptionPublicKey?: Uint8Array;
};

/** Type safety: enforce that the utxo is not encrypted */
export type Public = { isPublic: true };
export type PublicBaseUtxo = BaseUtxo & Public;
export type PublicUtxo = Utxo & Public;
export type PublicOutUtxo = OutUtxo & Public;

export type NullifierInputs = {
    signature: BN;
    /** hash of the utxo preimage */
    hash: BN254;
    merkleTreeLeafIndex: BN;
};

export type UtxoHashInputs = {
    owner: string;
    amounts: string[];
    assetsCircuitInput: string[];
    blinding: string;
    poolType: string;
    version: string;
    dataHash: string;
    metaHash: string;
    address: string;
};
