import { PublicKey } from "@solana/web3.js";

import {
  type as pick,
  number,
  string,
  array,
  boolean,
  literal,
  record,
  union,
  optional,
  nullable,
  coerce,
  instance,
  create,
  tuple,
  unknown,
  any,
  define,
} from "superstruct";
import type { Struct } from "superstruct";
import {
  TlvDataElement,
  decodeUtxoData,
  isValidTlvDataElement,
} from "./state/utxo-data";
import { PublicKey254, UtxoWithMerkleContext } from "./state";

// const TlvDataElementStruct = define("TlvDataElement", isValidTlvDataElement);

const PublicKeyFromString = coerce(
  instance(PublicKey),
  string(),
  (value) => new PublicKey(value)
);

const Base64EncodedUtxoDataResult = tuple([string(), literal("base64")]);

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
        sequence: number(),
      }),
      value,
    })
  );
}

/** Extra contextual information for RPC responses */
export type Context = {
  slot: number;
  sequence: number;
};
/**
 * RPC Response with extra contextual information
 */
export type RpcResponseAndContext<T> = {
  /** response context */
  context: Context;
  /** response value */
  value: T;
};

/**
 * @internal
 */
/// TODO: ensure consistency with photon
export const UtxoWithMerkleContextResult = pick({
  owner: PublicKeyFromString,
  lamports: number(),
  data: TlvFromBase64EncodedUtxoData,
  address: optional(PublicKeyFromString), // account

  leafIndex: number(), // bigint?
  hash: PublicKeyFromString,
  merkleTree: PublicKeyFromString,
});

/// These should be the actual structs returned from the rpc
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

export interface CompressedAccountInfoRpcResponse {
  data: Buffer;
  owner: PublicKey;
  utxoHash: string;
  merkleTree: PublicKey;
  slotUpdated: number;
  seq: number;
}

export interface CompressedAccountProofRpcResponse {
  utxoHash: string;
  merkleTree: PublicKey;
  slotUpdated: number;
  seq: number;
  proof: PublicKey[];
}

export type ProgramAccountsFilterOptions = {
  filters: Array<{
    memcmp: {
      offset: number;
      bytes: string; // base64
    };
  }>;
};

export interface CompressionApiInterface {
  /** Retrieve a utxo */
  getUtxo(
    utxoHash: PublicKey254,
    encoding?: string
  ): Promise<RpcResponseAndContext<UtxoWithMerkleContext>>;
  /** Retrieve the proof for a utxo */
  getUTXOProof(utxoHash: string): Promise<UtxoProofRpcResponse>;
  /** Retrieve a compressed account */
  getCompressedAccount(
    address: PublicKey,
    encoding?: "base64"
  ): Promise<CompressedAccountInfoRpcResponse>;

  /** Retrieve a recent Merkle proof for a compressed account */
  getCompressedProgramAccountProof(
    address: PublicKey
  ): Promise<CompressedAccountProofRpcResponse>;

  /** Retrieve all compressed accounts for a given owner */
  getCompressedAccounts( // GPA
    owner: PublicKey,
    encoding?: "base64",
    filters?: ProgramAccountsFilterOptions
  ): Promise<CompressedAccountInfoRpcResponse[]>;
}
