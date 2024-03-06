/// TODO: coerce directly as BN254fromString, sync interface with spec
import {
  Connection,
  ConnectionConfig,
  PublicKey,
  SolanaJSONRPCError,
} from '@solana/web3.js';
import {
  CompressedAccountMerkleProofResult,
  CompressedAccountResult,
  CompressedAccountsResult,
  CompressionApiInterface,
  GetCompressedAccountConfig,
  GetCompressedAccountsConfig,
  GetUtxoConfig,
  MerkleProofResult,
  UtxoResult,
  WithMerkleUpdateContext,
  jsonRpcResultAndContext,
} from './rpc-interface';
import {
  UtxoWithMerkleContext,
  MerkleContextWithMerkleProof,
  MerkleUpdateContext,
  BN254,
  PublicKeyToBN254,
} from './state';
import { array, create, nullable } from 'superstruct';
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
      slotUpdated: res.result.value.slotUpdated,
      seq: res.result.value.seq,
    };

    const value: UtxoWithMerkleContext = {
      owner: res.result.value.owner,
      lamports: res.result.value.lamports,
      data: res.result.value.data,
      hash: utxoHash,
      merkleTree: res.result.value.merkleTree,
      leafIndex: res.result.value.leafIndex,
      address: res.result.value.address,
      stateNullifierQueue: res.result.value.stateNullifierQueue,
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
      merkleProof: res.result.value.proof.map((proof) =>
        PublicKeyToBN254(proof),
      ),
      stateNullifierQueue: res.result.value.stateNullifierQueue,
      rootIndex: res.result.value.rootIndex,
    };
    return value;
  }

  /** Retrieve a compressed account */
  async getCompressedAccount(
    address: PublicKey,
    config?: GetCompressedAccountConfig,
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null> {
    const unsafeRes = await rpcRequest(
      this.rpcEndpoint,
      'getCompressedAccount',
      [address.toString(), config?.encoding || 'base64'],
    );
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(CompressedAccountResult)),
    );
    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get info about utxo ${address.toString()}`,
      );
    }

    if (res.result.value === null) {
      return null;
    }

    const context: MerkleUpdateContext = {
      slotUpdated: res.result.value.slotUpdated,
      seq: res.result.value.seq,
    };

    const value: UtxoWithMerkleContext = {
      owner: res.result.value.owner,
      lamports: res.result.value.lamports,
      data: res.result.value.data,
      hash: PublicKeyToBN254(res.result.value.hash),
      merkleTree: res.result.value.merkleTree,
      stateNullifierQueue: res.result.value.stateNullifierQueue,
      leafIndex: res.result.value.leafIndex,
      address,
    };
    return { context, value };
  }

  /** Retrieve a recent Merkle proof for a compressed account */
  async getCompressedAccountProof(
    address: PublicKey,
  ): Promise<MerkleContextWithMerkleProof | null> {
    const unsafeRes = await rpcRequest(
      this.rpcEndpoint,
      'getCompressedAccountProof',
      [address.toString()],
    );
    const res = create(
      unsafeRes,
      jsonRpcResultAndContext(nullable(CompressedAccountMerkleProofResult)),
    );
    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get proof for compressed account ${address.toString()}`,
      );
    }
    if (res.result.value === null) {
      return null;
    }

    const value = {
      hash: PublicKeyToBN254(res.result.value.utxoHash),
      merkleTree: res.result.value.merkleTree,
      leafIndex: res.result.value.leafIndex,
      merkleProof: res.result.value.proof.map((proof) =>
        PublicKeyToBN254(proof),
      ),
      stateNullifierQueue: res.result.value.stateNullifierQueue,
      rootIndex: res.result.value.rootIndex,
    };

    return value;
  }

  /** Retrieve all compressed accounts for a given owner */
  async getCompressedAccounts(
    owner: PublicKey,
    config?: GetCompressedAccountsConfig,
  ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]> {
    const unsafeRes = await rpcRequest(
      this.rpcEndpoint,
      'getCompressedAccounts',
      [owner.toString(), config?.encoding || 'base64', config?.filters],
    );
    const baseSchema = array(CompressedAccountsResult);
    const res = create(unsafeRes, jsonRpcResultAndContext(baseSchema));
    if ('error' in res) {
      throw new SolanaJSONRPCError(
        res.error,
        `failed to get compressed accounts for owner ${owner.toString()}`,
      );
    }

    const values = res.result.value.map((value) => {
      const context: MerkleUpdateContext = {
        slotUpdated: value.slotUpdated,
        seq: value.seq,
      };
      return {
        context,
        value: {
          owner: value.owner,
          lamports: value.lamports,
          data: value.data,
          hash: PublicKeyToBN254(value.hash),
          merkleTree: value.merkleTree,
          stateNullifierQueue: value.stateNullifierQueue,
          leafIndex: value.leafIndex,
          address: value.address,
        },
      };
    });
    return values;
  }
}
