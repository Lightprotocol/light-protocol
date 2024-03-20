import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';

/// TODO: Consider flattening and implementing an IR in Beet.
export interface PackedCompressedAccountWithMerkleContext {
  compressedAccount: CompressedAccount;
  indexMtAccount: number; // u8
  indexNullifierArrayAccount: number; // u8
  leafIndex: number; // u32 FIXME: switch on-chain to u64.
  // Missing: hash
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 * */
export interface CompressedAccount {
  /** Public key of program or user that owns the account */
  owner: PublicKey;
  /** Lamports attached to the account */
  lamports: BN; // u64 // FIXME: optional
  /**
   * TODO: Implement address functionality. Optional unique account ID that is
   * persistent across transactions.
   */
  address: PublicKey | null; // Option<PublicKey>
  /** Optional data attached to the account */
  data: CompressedAccountData | null; // Option<CompressedAccountData>
}

export interface CompressedAccountData {
  discriminator: number[]; // [u8; 8] // TODO: test with uint8Array instead
  data: Buffer; // bytes
  dataHash: number[]; // [u8; 32]
}

export interface PublicTransactionEvent {
  inputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
  outputAccountHashes: number[][]; // Vec<[u8; 32]>
  inputCompressedAccounts: PackedCompressedAccountWithMerkleContext[];
  outputCompressedAccounts: CompressedAccount[];
  outputStateMerkleTreeAccountIndices: Uint8Array; // bytes
  outputLeafIndices: number[]; // Vec<u32>
  relayFee: BN | null; // Option<u64>
  deCompressAmount: BN | null; // Option<u64>
  pubkeyArray: PublicKey[]; // Vec<PublicKey>
  message: Uint8Array | null; // Option<bytes>
}

export interface InstructionDataTransfer {
  proof: CompressedProof | null; // Option<CompressedProof>
  inputRootIndices: number[]; // Vec<u16>
  inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
  outputCompressedAccounts: CompressedAccount[];
  outputStateMerkleTreeAccountIndices: Buffer; // bytes // FIXME: into Vec<u8> on-chain
  relayFee: BN | null; // Option<u64>
}

export interface CompressedProof {
  a: number[]; // [u8; 32]
  b: number[]; // [u8; 64]
  c: number[]; // [u8; 32]
}
