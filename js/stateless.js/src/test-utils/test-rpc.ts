/// TODO: mock properly for all methods, build merkletree locally

import { Connection, ConnectionConfig, PublicKey } from "@solana/web3.js";
import {
  CompressedAccountInfoRpcResponse,
  CompressionApiInterface,
} from "../rpc-interface";
import { Rpc, createRpc } from "../rpc";

/**
 * Mirrors the {@link createRpc} utility
 * @param    endpointOrWeb3JsConnection
 * @param    config
 * @returns  {TestRpc}
 *
 */
export function createTestRpc(
  endpointOrWeb3JsConnection: string | Connection,
  config?: ConnectionConfig
): TestRpc {
  if (typeof endpointOrWeb3JsConnection === "string") {
    return new TestRpc(endpointOrWeb3JsConnection, config);
  }
  return new TestRpc(endpointOrWeb3JsConnection.rpcEndpoint, config);
}

/**
 * A test-RPC that mocks the {@link CompressionApiInterface} interface.
 * Ideal for local development and unit tests.
 * Fetches all txs from connection.rpcEndpoint and builds a local merkle tree.
 * @extends Rpc
 */
// TODO: consider adding a pure mock without local validator - e.g. mock through caching the relevant tree state
// TODO: add a test Forrester Node implementation
export class TestRpc extends Rpc implements CompressionApiInterface {
  constructor(endpoint: string, config?: ConnectionConfig) {
    super(endpoint, config);
  }

  async getCompressedAccountInfo(
    _address: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse> {
    return { compressedAccountInfo: "mockCompressedAccountInfoString" };
  }

  async getCompressedProgramAccounts(
    _programId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse[]> {
    return [{ compressedAccountInfo: "mockCompressedAccountInfoString" }];
  }
}
