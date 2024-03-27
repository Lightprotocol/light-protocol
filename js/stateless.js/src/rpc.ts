import {
  Connection,
  ConnectionConfig,
  SolanaJSONRPCError,
  PublicKey,
} from '@solana/web3.js';
import {
  CompressedProofWithContext,
  CompressionApiInterface,
  GetUtxoConfig,
  MerkleProofResult,
  WithMerkleUpdateContext,
  jsonRpcResultAndContext,
} from './rpc-interface';
import {
  MerkleContextWithMerkleProof,
  MerkleUpdateContext,
  BN254,
  bn,
  CompressedAccountWithMerkleContext,
  createBN254,
} from './state';
import { create, nullable } from 'superstruct';
import { toCamelCase } from './utils/conversion';

export function createRpc(
  endpointOrWeb3JsConnection: string | Connection,
  config?: ConnectionConfig,
): Rpc {
  if (typeof endpointOrWeb3JsConnection === 'string') {
    return new Rpc(endpointOrWeb3JsConnection, undefined, config);
  }
  return new Rpc(endpointOrWeb3JsConnection.rpcEndpoint, undefined, config);
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
  constructor(
    endpoint: string,
    // TODO: implement
    proverEndpoint?: string,
    config?: ConnectionConfig,
  ) {
    super(endpoint, config || 'confirmed');
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
      hash: utxoHash as any, // FIXME
      merkleTree: res.result.value.merkleTree,
      leafIndex: res.result.value.leafIndex,
      merkleProof: res.result.value.proof.map(proof => createBN254(proof)),
      nullifierQueue: res.result.value.nullifierQueue,
      rootIndex: res.result.value.rootIndex,
    };
    return value;
  }

  /** Retrieve a utxo with context */
  async getUtxos(
    owner: PublicKey,
    config?: GetUtxoConfig,
  ): Promise<WithMerkleUpdateContext<CompressedAccountWithMerkleContext>[]> {
    const unsafeRes = await rpcRequest(this.rpcEndpoint, 'getUtxos', [
      owner.toString(),
      config?.encoding || 'base64',
    ]);
    // const res: any = create(
    //     unsafeRes,
    //     jsonRpcResultAndContext(nullable(UtxosResult)),
    // );
    const res: any = unsafeRes;

    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get info about utxos for owner ${owner.toString()}`,
      );
    }

    if (res.result.value === null) {
      return [];
    }

    const utxosWithMerkleContext: WithMerkleUpdateContext<CompressedAccountWithMerkleContext>[] =
      res.result.value.map((account: any) => {
        const context: MerkleUpdateContext = {
          slotCreated: account.slotCreated,
          seq: account.seq,
        };

        const value: CompressedAccountWithMerkleContext = {
          owner: owner,
          lamports: bn(account.lamports),
          data: account.data,
          hash: account.hash, // Assuming utxoHash is defined elsewhere or needs to be handled per utxo basis
          merkleTree: account.merkleTree,
          leafIndex: account.leafIndex,
          address: account.address || null,
          nullifierQueue: account.nullifierQueue,
        };

        return { context, value };
      });
    return utxosWithMerkleContext;
  }
  async getValidityProof(
    /// TODO: Implement
    utxoHashes: BN254[],
  ): Promise<CompressedProofWithContext> {
    throw new Error('Method not implemented.');
  }
}
