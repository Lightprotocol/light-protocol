import {
  Connection,
  ConnectionConfig,
  PublicKey,
  SolanaJSONRPCError,
} from "@solana/web3.js";
import {
  CompressedAccountInfoRpcResponse,
  CompressedAccountProofRpcResponse,
  CompressionApiInterface,
  ProgramAccountsFilterOptions,
  RpcResponseAndContext,
  UtxoProofRpcResponse,
  UtxoResult,
  UtxoRpcResponse,
  UtxoWithMerkleContextResult,
  jsonRpcResultAndContext,
} from "./rpc-interface";
import { PublicKey254, Utxo, UtxoWithMerkleContext } from "./state";
import { decodeUtxoData } from "./state/utxo-data";
import { create, nullable } from "superstruct";

export function createRpc(
  endpointOrWeb3JsConnection: string | Connection,
  config?: ConnectionConfig
): Rpc {
  if (typeof endpointOrWeb3JsConnection === "string") {
    return new Rpc(endpointOrWeb3JsConnection, config);
  }
  return new Rpc(endpointOrWeb3JsConnection.rpcEndpoint, config);
}

const rpcRequest = async (
  rpcEndpoint: string,
  method: string,
  params: any[] = []
): Promise<any> => {
  const body = JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: method,
    params: params,
  });

  const response = await fetch(rpcEndpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body,
  });

  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }

  return await response.json();
};

export class Rpc extends Connection implements CompressionApiInterface {
  constructor(endpoint: string, config?: ConnectionConfig) {
    super(endpoint, config);
  }

  /** Retrieve a utxo with context */
  async getUtxo(
    utxoHash: PublicKey254,
    _encoding?: string
  ): Promise<RpcResponseAndContext<UtxoWithMerkleContext>> {
    const unsafeRes = rpcRequest(this.rpcEndpoint, "getUtxo", [
      utxoHash.toString(),
    ]);
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(UtxoWithMerkleContextResult))
    );
    if ("error" in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get info about utxo ${utxoHash.toString()}`
      );
    }
    return res.result;
  }

  /** Retrieve the proof for a utxo */
  async getUTXOProof(utxoHash: string): Promise<UtxoProofRpcResponse> {}

  /** Retrieve a compressed account */
  async getCompressedAccount(
    address: PublicKey,
    encoding?: string
  ): Promise<CompressedAccountInfoRpcResponse> {}

  /** Retrieve a recent Merkle proof for a compressed account */
  getCompressedProgramAccountProof(
    address: PublicKey
  ): Promise<CompressedAccountProofRpcResponse> {}

  /** Retrieve all compressed accounts for a given owner */
  async getCompressedAccounts(
    owner: PublicKey,
    encoding?: "base64",
    filters?: ProgramAccountsFilterOptions
  ): Promise<CompressedAccountInfoRpcResponse[]> {
    return rpcRequest(this.rpcEndpoint, "getCompressedProgramAccounts", [
      owner.toString(),
    ]);
  }
}
