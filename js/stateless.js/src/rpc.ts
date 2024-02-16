import { Connection, ConnectionConfig, PublicKey } from "@solana/web3.js";
import {
  CompressedAccountInfoRpcResponse,
  CompressionApiInterface,
} from "./rpc-interface";

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

  async getCompressedAccountInfo(
    address: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse> {
    return rpcRequest(this.rpcEndpoint, "getCompressedAccountInfo", [
      address.toString(),
    ]);
  }

  async getCompressedProgramAccounts(
    programId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse[]> {
    return rpcRequest(this.rpcEndpoint, "getCompressedProgramAccounts", [
      programId.toString(),
    ]);
  }
}
