import { PublicKey } from "@solana/web3.js";

export type Base58EncodedString = string;

export interface CompressedAccountInfoRpcResponse {
  data: Buffer;
  owner: PublicKey;
  utxoHash: string;
  merkleTree: PublicKey;
  slot_updated: number;
  seq: number;
}

export interface UtxoRpcResponse {
  data: Buffer;
  owner: PublicKey;
  blinding: string; // TODO: to leafIndex
  slotUpdated: number;
  seq: number;
  address?: PublicKey;
}

export interface UtxoProofRpcResponse {
  merkleTree: PublicKey;
  proof: PublicKey[];
  slotUpdated: number;
  seq: number;
}

export interface CompressionApiInterface {
  /** Retrieve a utxo */
  getUtxo(utxoHash: string, encoding?: string): Promise<UtxoRpcResponse>;
  /** Retrieve the proof for a utxo */
  getUTXOProof(utxoHash: string): Promise<UtxoProofRpcResponse>;

  getCompressedAccount(
    address: PublicKey,
    encoding?: string
  ): Promise<CompressedAccountInfoRpcResponse>;
  getCompressedProgramAccounts(
    assetId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse[]>;
}
