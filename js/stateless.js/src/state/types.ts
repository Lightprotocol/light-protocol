/**
 * Implements the IDL types for the stateless.js program for typesafety
 * TODO: unify with core types as beet implementation or similar.
 */

import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { Utxo } from './utxo';

export type PublicTransactionEvent_IdlType = {
  inUtxos: Utxo_IdlType[];
  outUtxos: Utxo_IdlType[];
  deCompressedAmount: BN | null;
  outUtxoIndices: BN[];
  relayFee: BN | null;
  message: Uint8Array | null;
};

/// Utxo types

export type Utxo_IdlType = {
  owner: PublicKey;
  blinding: number[];
  lamports: BN;
  address: PublicKey | null;
  data: Tlv_IdlType | null;
};

export type Tlv_IdlType = {
  tlvElements: TlvDataElement_IdlType[];
};

export type TlvDataElement_IdlType = {
  discriminator: number[];
  owner: PublicKey;
  data: Uint8Array;
  dataHash: number[];
};

export type InUtxoTuple_IdlType = {
  inUtxo: Utxo_IdlType; // think we need to attach leafIndex as blinding here!
  indexMtAccount: number;
  indexNullifierArrayAccount: number;
};

export type OutUtxoTuple_IdlType = {
  outUtxo: Utxo;
  indexMtAccount: number;
};

/// Serial types

export type TlvSerializable_IdlType = {
  tlvElements: TlvDataElementSerializable_IdlType[];
};

export type TlvDataElementSerializable_IdlType = {
  discriminator: number[];
  owner: number;
  data: Uint8Array;
  dataHash: number[];
};

export type InUtxoSerializable_IdlType = {
  owner: number;
  leafIndex: number;
  lamports: number;
  address: PublicKey | null;
  data: TlvSerializable_IdlType | null;
};

export type OutUtxoSerializable_IdlType = {
  owner: number;
  lamports: number;
  address: PublicKey | null;
  data: TlvSerializable_IdlType | null;
};

export type InUtxoSerializableTuple_IdlType = {
  inUtxoSerializable: InUtxoSerializable_IdlType;
  indexMtAccount: number;
  indexNullifierArrayAccount: number;
};

export type OutUtxoSerializableTuple = {
  outUtxoSerializable: OutUtxoSerializable_IdlType;
  indexMtAccount: number;
};

export type CompressedProof_IdlType = {
  a: number[];
  b: number[];
  c: number[];
};
