import {
  Connection,
  ConnectionConfig,
  SolanaJSONRPCError,
  PublicKey,
} from '@solana/web3.js';
import {
  CompressionApiInterface,
  GetUtxoConfig,
  MerkleProofResult,
  UtxoResult,
  UtxosResult,
  WithMerkleUpdateContext,
  jsonRpcResultAndContext,
} from './rpc-interface';
import {
  UtxoWithMerkleContext,
  MerkleContextWithMerkleProof,
  MerkleUpdateContext,
  BN254,
  bn,
  createBN254,
} from './state';
import { create, nullable } from 'superstruct';
import { toCamelCase } from './utils/conversion';

export function createRpc(
  endpointOrWeb3JsConnection: string | Connection,
  config?: ConnectionConfig,
): Rpc {
  if (typeof endpointOrWeb3JsConnection === 'string') {
    return new Rpc(endpointOrWeb3JsConnection, config);
  }
  return new Rpc(endpointOrWeb3JsConnection.rpcEndpoint, config);
}

const rpcRequest = async (
  rpcEndpoint: string,
  method: string,
  params: any[] = [],
  convertToCamelCase = true,
): Promise<any> => {
  const body = JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: method,
    params: params,
  });

  const response = await fetch(rpcEndpoint, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body,
  });

  if (!response.ok) {
    throw new Error(`HTTP error! status: ${response.status}`);
  }

  if (convertToCamelCase) {
    const res = await response.json();
    return toCamelCase(res);
  }
  return await response.json();
};

export class Rpc extends Connection implements CompressionApiInterface {
  constructor(endpoint: string, config?: ConnectionConfig) {
    super(endpoint, config);
  }

  /** Retrieve a utxo with context */
  async getUtxo(
    utxoHash: BN254,
    config?: GetUtxoConfig,
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null> {
    const unsafeRes = await rpcRequest(this.rpcEndpoint, 'getUtxo', [
      utxoHash.toString(),
      config?.encoding || 'base64',
    ]);
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(UtxoResult)),
    );

    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get info about utxo ${utxoHash.toString()}`,
      );
    }

    if (res.result.value === null) {
      return null;
    }

    const context: MerkleUpdateContext = {
      slotCreated: res.result.value.slotCreated,
      seq: res.result.value.seq,
    };

    const value: UtxoWithMerkleContext = {
      owner: res.result.value.owner,
      lamports: bn(res.result.value.lamports),
      data: { tlvElements: res.result.value.data },
      hash: utxoHash,
      merkleTree: res.result.value.merkleTree,
      leafIndex: bn(res.result.value.leafIndex),
      address: res.result.value.address || null,
      nullifierQueue: res.result.value.nullifierQueue,
    };

    return { context, value };
  }

  /** Retrieve the proof for a utxo */
  async getUtxoProof(
    utxoHash: BN254,
  ): Promise<MerkleContextWithMerkleProof | null> {
    const unsafeRes = await rpcRequest(this.rpcEndpoint, 'getUtxoProof', [
      utxoHash.toString(),
    ]);
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(MerkleProofResult)),
    );
    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get proof for utxo ${utxoHash.toString()}`,
      );
    }
    if (res.result.value === null) {
      return null;
    }
    const value: MerkleContextWithMerkleProof = {
      hash: utxoHash,
      merkleTree: res.result.value.merkleTree,
      leafIndex: res.result.value.leafIndex,
      merkleProof: res.result.value.proof.map((proof) => createBN254(proof)),
      nullifierQueue: res.result.value.nullifierQueue,
      rootIndex: res.result.value.rootIndex,
    };
    return value;
  }

  /** Retrieve a utxo with context */
  async getUtxos(
    owner: PublicKey,
    config?: GetUtxoConfig,
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]> {
    const unsafeRes = await rpcRequest(this.rpcEndpoint, 'getUtxos', [
      owner.toString(),
      config?.encoding || 'base64',
    ]);
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(UtxosResult)),
    );

    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get info about utxos for owner ${owner.toString()}`,
      );
    }

    if (res.result.value === null) {
      return [];
    }

    const utxosWithMerkleContext: WithMerkleUpdateContext<UtxoWithMerkleContext>[] =
      res.result.value.map((utxo) => {
        const context: MerkleUpdateContext = {
          slotCreated: utxo.slotCreated,
          seq: utxo.seq,
        };

        const value: UtxoWithMerkleContext = {
          owner: owner,
          lamports: bn(utxo.lamports),
          data: { tlvElements: utxo.data },
          hash: utxo.hash, // Assuming utxoHash is defined elsewhere or needs to be handled per utxo basis
          merkleTree: utxo.merkleTree,
          leafIndex: bn(utxo.leafIndex),
          address: utxo.address || null,
          nullifierQueue: utxo.nullifierQueue,
        };

        return { context, value };
      });
    return utxosWithMerkleContext;
  }
}
