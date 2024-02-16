import { PublicKey } from "@solana/web3.js";
import { bigint254 } from "./bigint254";

// export type TlvDataElementSerializable = {
//     discriminator: [u8; 8],
//     owner: u8,
//     data: Vec<u8>,
//     data_hash: [u8; 32],
// }

/** Describes the generic utxo details applicable to every utxo */
export type Utxo = {
  /** Public key of the user or program owning the utxo */
  owner: PublicKey;
  /** Optional data associated with the utxo */
  data: number[];
  /** lamports attached to utxo, default 0*/
  lamports: bigint;
  /**
   * Optional persistent id of the account.
   * Utxos are immutable and their hash is epheremeal (i.e. only associative to a single utxo); each tx invalidates inUtxos (current state) and
   * creates outUtxos (new state).
   * 'address' helps maintain a persistent unique id across transactions.
   * This is useful for mimicking compressed PDAs and non-fungible tokens
   */
  /// TODO: implement address functionality
  /// This would go into the data field as first tlv.
  // address?: bigint254;
};

/** Utxo that had previously been inserted into a state Merkle tree */
export type UtxoWithMerkleProof = Utxo & {
  /** Index of 'hash' as inserted into the Merkle tree. Max safe tree depth using number type would be **52, roughly 4.5 x 10^15 leaves */
  leafIndex: bigint | number;
  /** Unique identifier and commitment to the utxo preimage, is inserted as leaf into state tree */
  hash: bigint254;
  /** Numerical identifier of the Merkle tree which the 'hash' is part of */
  merkletreeId: bigint | number;
  /** Proof path attached to the utxo. Can be reconstructed using event history */
  merkleProof: string[];
};

// decode utxo (deserialize, serialize)
