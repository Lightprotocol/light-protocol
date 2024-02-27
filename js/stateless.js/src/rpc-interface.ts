import { PublicKey, DataSizeFilter, MemcmpFilter } from "@solana/web3.js";

import {
  type as pick,
  number,
  string,
  array,
  literal,
  union,
  optional,
  coerce,
  instance,
  create,
  tuple,
  unknown,
  any,
} from "superstruct";
import type { Struct } from "superstruct";
import {
  TlvDataElement,
  decodeUtxoData,
  isValidTlvDataElement,
} from "./state/utxo-data";
import {
  MerkleContext,
  MerkleContextWithMerkleProof,
  MerkleUpdateContext,
  PublicKey254,
  UtxoWithMerkleContext,
} from "./state";

export type GetCompressedAccountsFilter = MemcmpFilter | DataSizeFilter;

export type GetUtxoConfig = {
  encoding?: string;
};
export type GetCompressedAccountConfig = GetUtxoConfig;

export type GetCompressedAccountsConfig = {
  encoding?: string;
  filters?: GetCompressedAccountsFilter[];
};

export type WithMerkleUpdateContext<T> = {
  /** merkle update context */
  context: MerkleUpdateContext;
  /** response value */
  value: T;
};

/**
 * @internal
 */
const PublicKeyFromString = coerce(
  instance(PublicKey),
  string(),
  (value) => new PublicKey(value)
);

/**
 * @internal
 */
const Base64EncodedUtxoDataResult = tuple([string(), literal("base64")]);

/**
 * @internal
 */
const TlvFromBase64EncodedUtxoData = coerce(
  instance(Array<TlvDataElement>),
  Base64EncodedUtxoDataResult,
  (value) => {
    const decodedData = decodeUtxoData(Buffer.from(value[0], "base64"));
    if (decodedData.every(isValidTlvDataElement)) {
      return decodedData;
    } else {
      throw new Error("Invalid TlvDataElement structure");
    }
  }
);

/**
 * @internal
 */
export function createRpcResult<T, U>(result: Struct<T, U>) {
  return union([
    pick({
      jsonrpc: literal("2.0"),
      id: string(),
      result,
    }),
    pick({
      jsonrpc: literal("2.0"),
      id: string(),
      error: pick({
        code: unknown(),
        message: string(),
        data: optional(any()),
      }),
    }),
  ]);
}

/**
 * @internal
 */
const UnknownRpcResult = createRpcResult(unknown());

/**
 * @internal
 */
export function jsonRpcResult<T, U>(schema: Struct<T, U>) {
  return coerce(createRpcResult(schema), UnknownRpcResult, (value) => {
    if ("error" in value) {
      return value;
    } else {
      return {
        ...value,
        result: create(value.result, schema),
      };
    }
  });
}

/**
 * @internal
 */
export function jsonRpcResultAndContext<T, U>(value: Struct<T, U>) {
  return jsonRpcResult(
    pick({
      context: pick({
        slot: number(),
      }),
      value,
    })
  );
}

/**
 * @internal
 */
/// Utxo with merkle context
export const UtxoResult = pick({
  owner: PublicKeyFromString,
  lamports: number(),
  data: TlvFromBase64EncodedUtxoData,
  address: optional(PublicKeyFromString), // account
  leafIndex: number(), // bigint?
  merkleTree: PublicKeyFromString,
  slotUpdated: number(),
  seq: number(),
});

/**
 * @internal
 */
export const CompressedAccountResult = pick({
  owner: PublicKeyFromString,
  lamports: number(),
  data: TlvFromBase64EncodedUtxoData,
  hash: PublicKeyFromString,
  leafIndex: number(),
  merkleTree: PublicKeyFromString,
  slotUpdated: number(),
  seq: number(),
});

/**
 * @internal
 */
export const CompressedAccountsResult = pick({
  owner: PublicKeyFromString,
  address: PublicKeyFromString,
  lamports: number(),
  data: TlvFromBase64EncodedUtxoData,
  hash: PublicKeyFromString,
  leafIndex: number(),
  merkleTree: PublicKeyFromString,
  slotUpdated: number(),
  seq: number(),
});

/**
 * @internal
 */
export const MerkleProofResult = pick({
  merkleTree: PublicKeyFromString,
  leafIndex: number(),
  proof: array(PublicKeyFromString),
});

/**
 * @internal
 */
export const CompressedAccountMerkleProofResult = pick({
  utxoHash: PublicKeyFromString,
  merkleTree: PublicKeyFromString,
  leafIndex: number(),
  proof: array(PublicKeyFromString),
});

export interface CompressionApiInterface {
  /** Retrieve a utxo */
  getUtxo(
    utxoHash: PublicKey254,
    config?: GetUtxoConfig
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null>;
  /** Retrieve the proof for a utxo */
  getUtxoProof(utxoHash: PublicKey254): Promise<MerkleContext | null>;
  /** Retrieve a compressed account */
  getCompressedAccount(
    address: PublicKey,
    config?: GetCompressedAccountConfig
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null>;
  /** Retrieve a recent Merkle proof for a compressed account */
  getCompressedAccountProof(
    address: PublicKey
  ): Promise<MerkleContextWithMerkleProof | null>;
  /** Retrieve all compressed accounts for a given owner */
  getCompressedAccounts( // GPA
    owner: PublicKey,
    config?: GetCompressedAccountsConfig
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]>;
}
