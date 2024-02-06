import {OutUtxo, Utxo} from "utxo/utxo-types";
import {PublicKey} from "@solana/web3.js";
import {BN254} from "utxo/bn254";
import {Idl} from "@coral-xyz/anchor";

export type PlaceHolderTData = any;
/** Program-owned utxo that had previously been inserted into a state Merkle tree */
export type ProgramUtxo<TData extends PlaceHolderTData> = Omit<
    Utxo,
    "owner"
> & {
    /** Public key of program that owns the utxo */
    owner: PublicKey;
    /** Data assigned to the utxo */
    data: TData;
    /** Hash of 'data' */
    dataHash: BN254;
    /** psp idl */
    ownerIdl: Idl; /// TODO: remove from utxo (waste of space)
};

/** Program-owned utxo that is not inserted into the state tree yet. */
export type ProgramOutUtxo<TData extends PlaceHolderTData> = Omit<
    OutUtxo,
    "owner"
> & {
    /** Public key of program that owns the utxo */
    owner: PublicKey;
    /** Data assigned to the utxo */
    data: TData;
    /** Hash of 'data' */
    dataHash: BN254;
    /** psp idl */
    ownerIdl: Idl; /// TODO: remove from utxo (waste of space)
};
