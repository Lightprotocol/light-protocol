import {
    Connection,
    ConnectionConfig,
    SolanaJSONRPCError,
    PublicKey,
} from '@solana/web3.js';
import { Buffer } from 'buffer';

import {
    BalanceResult,
    CompressedAccountResult,
    CompressedAccountsByOwnerResult,
    CompressedProofWithContext,
    CompressedTokenAccountsByOwnerOrDelegateResult,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    HealthResult,
    MerkeProofResult,
    MultipleCompressedAccountsResult,
    ParsedTokenAccount,
    SlotResult,
    jsonRpcResult,
    jsonRpcResultAndContext,
} from './rpc-interface';
import {
    MerkleContextWithMerkleProof,
    BN254,
    bn,
    CompressedAccountWithMerkleContext,
    encodeBN254toBase58,
    createCompressedAccountWithMerkleContext,
    createMerkleContext,
    TokenData,
} from './state';
import { array, create, nullable } from 'superstruct';
import { defaultTestStateTreeAccounts } from './constants';
import { BN } from '@coral-xyz/anchor';

export interface HexInputsForProver {
    roots: string[];
    inPathIndices: number[];
    inPathElements: string[][];
    leaves: string[];
}
import { toCamelCase } from './utils/conversion';

import {
    proofFromJsonStruct,
    negateAndCompressProof,
} from './utils/parse-validity-proof';
import { getTestRpc, getParsedEvents } from '@lightprotocol/test-helpers';

export function createRpc(
    endpointOrWeb3JsConnection: string | Connection = 'http://127.0.0.1:8899',
    compressionApiEndpoint: string = 'http://localhost:8784',
    proverEndpoint: string = 'http://localhost:3001',
    config?: ConnectionConfig,
): Rpc {
    const endpoint =
        typeof endpointOrWeb3JsConnection === 'string'
            ? endpointOrWeb3JsConnection
            : endpointOrWeb3JsConnection.rpcEndpoint;
    return new Rpc(endpoint, compressionApiEndpoint, proverEndpoint, config);
}

/** @internal */
export const rpcRequest = async (
    rpcEndpoint: string,
    method: string,
    params: any = [], // TODO: array?
    convertToCamelCase = true,
): Promise<any> => {
    const body = JSON.stringify({
        jsonrpc: '2.0',
        id: 'test-account',
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

/// TODO: replace with dynamic nullifierQueue
const mockNullifierQueue = defaultTestStateTreeAccounts().nullifierQueue;

/**
 * @internal
 * This only works with uncranked state trees in local test environments.
 * TODO: implement as seq MOD rootHistoryArray.length, or move to indexer
 */
export const getRootSeq = async (rpc: Rpc): Promise<number> => {
    const events = (await getParsedEvents(rpc)).reverse();
    const leaves: number[][] = [];
    for (const event of events) {
        for (
            let index = 0;
            index < event.outputCompressedAccounts.length;
            index++
        ) {
            const hash = event.outputCompressedAccountHashes[index];

            leaves.push(hash);
        }
    }
    /// This only
    const rootSeq = leaves.length;
    return rootSeq;
};

/**
 * @internal
 * convert BN to hex with '0x' prefix
 */
export function toHex(bn: BN) {
    return '0x' + bn.toString('hex');
}

export class Rpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;

    constructor(
        endpoint: string,
        compressionApiEndpoint: string,
        proverEndpoint: string,
        config?: ConnectionConfig,
    ) {
        super(endpoint, config || 'confirmed');
        this.compressionApiEndpoint = compressionApiEndpoint;
        this.proverEndpoint = proverEndpoint;
    }

    async getCompressedAccount(
        hash: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccount',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(nullable(CompressedAccountResult)),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get info for compressed account ${hash.toString()}`,
            );
        }
        if (res.result.value === null) {
            return null;
        }
        const item = res.result.value;
        const account = createCompressedAccountWithMerkleContext(
            createMerkleContext(
                item.tree!,
                mockNullifierQueue,
                item.hash.toArray(undefined, 32),
                item.leafIndex,
            ),
            item.owner,
            bn(item.lamports),
            // TODO: fix. add typesafety to the rest
            item.data
                ? {
                      discriminator: item.discriminator.toArray('le', 8),
                      data: Buffer.from(item.data, 'base64'),
                      dataHash: item.dataHash!.toArray('le', 32),
                  }
                : undefined,

            item.address || undefined,
        );
        return account;
    }

    async getCompressedBalance(hash: BN254): Promise<BN | null> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedBalance',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(unsafeRes, jsonRpcResultAndContext(BalanceResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get balance for compressed account ${hash.toString()}`,
            );
        }
        if (res.result.value === null) {
            return null;
        }

        return bn(res.result.value);
    }

    /** Retrieve the merkle proof for a compressed account */
    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccountProof',
            encodeBN254toBase58(hash),
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(MerkeProofResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get proof for compressed account ${hash.toString()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get proof for compressed account ${hash.toString()}`,
            );
        }

        const proofWithoutRoot = res.result.value.proof.slice(0, -1);
        const root = res.result.value.proof[res.result.value.proof.length - 1];
        const rootIndex = await getRootSeq(this);

        const value: MerkleContextWithMerkleProof = {
            hash: res.result.value.hash.toArray(undefined, 32),
            merkleTree: res.result.value.merkleTree,
            leafIndex: res.result.value.leafIndex,
            merkleProof: proofWithoutRoot,
            nullifierQueue: mockNullifierQueue, // TODO: use nullifierQueue from indexer
            rootIndex, // TODO: use root index from indexer
            root, // TODO: use root from indexer
        };
        return value;
    }

    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[] | null> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleCompressedAccounts',
            hashes.map(hash => encodeBN254toBase58(hash)),
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(MultipleCompressedAccountsResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get info for compressed accounts ${hashes.map(hash => encodeBN254toBase58(hash)).join(', ')}`,
            );
        }
        if (res.result.value === null) {
            return null;
        }
        const accounts: CompressedAccountWithMerkleContext[] = [];
        res.result.value.items.map((item: any) => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    mockNullifierQueue,
                    item.hash.toArray(undefined, 32),
                    item.leafIndex,
                ),
                item.owner,
                bn(item.lamports),
                item.data && {
                    /// TODO: validate whether we need to convert to 'le' here
                    discriminator: item.discriminator.toArray('le', 8),
                    data: Buffer.from(item.data, 'base64'),
                    dataHash: item.dataHash.toArray('le', 32), //FIXME: need to calculate the hash or return from server
                },
                item.address,
            );
            accounts.push(account);
        });

        return accounts;
    }

    /** Retrieve the merkle proof for a compressed account */
    async getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]> {
        /// TODO: remove this once root is returned from indexer
        const testRpc = await getTestRpc(
            this.rpcEndpoint,
            this.compressionApiEndpoint,
            this.proverEndpoint,
        );
        const testProofInfo =
            await testRpc.getMultipleCompressedAccountProofs(hashes);

        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleCompressedAccountProofs',
            hashes.map(hash => encodeBN254toBase58(hash)),
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(array(MerkeProofResult)),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get proofs for compressed accounts ${hashes.map(hash => encodeBN254toBase58(hash)).join(', ')}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get proofs for compressed accounts ${hashes.map(hash => encodeBN254toBase58(hash)).join(', ')}`,
            );
        }

        const merkleProofs: MerkleContextWithMerkleProof[] = [];

        const rootIndex = await getRootSeq(this);

        for (const proof of res.result.value) {
            const proofWithoutRoot: BN[] = proof.proof.slice(0, -1);

            // const root = proof.proof[proof.proof.length - 1];
            const value: MerkleContextWithMerkleProof = {
                hash: proof.hash.toArray(undefined, 32),
                merkleTree: proof.merkleTree,
                leafIndex: proof.leafIndex,
                merkleProof: proofWithoutRoot,
                nullifierQueue: mockNullifierQueue, // TODO: use nullifierQueue from indexer
                rootIndex, // TODO: use root index from indexer
                root: bn(testProofInfo[res.result.value.indexOf(proof)].root), // TODO: use root from indexer
            };
            merkleProofs.push(value);
        }
        /// TODO: switch back to using photon merkle proofs once fixed
        return testProofInfo;
    }

    async getCompressedAccountsByOwner(
        owner: PublicKey,
    ): Promise<CompressedAccountWithMerkleContext[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccountsByOwner',
            { owner: owner.toBase58() },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(CompressedAccountsByOwnerResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get info for compressed accounts owned by ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            return [];
        }
        const accounts: CompressedAccountWithMerkleContext[] = [];
        /// TODO: clean up. Make typesafe
        res.result.value.items.map((item: any) => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    mockNullifierQueue,
                    item.hash.toArray(undefined, 32),
                    item.leafIndex,
                ),
                item.owner,
                bn(item.lamports),
                item.data && {
                    discriminator: item.discriminator.toArray('le', 8),
                    data: Buffer.from(item.data, 'base64'),
                    dataHash: item.dataHash.toArray('le', 32), //FIXME: need to calculate the hash or return from server
                },
                item.address,
            );

            accounts.push(account);
        });

        return accounts;
    }

    /**
     * Retrieves a validity proof for compressed accounts, proving their
     * existence in their respective state trees.
     *
     * Allows verifiers to verify state validity without recomputing the merkle
     * proof path, therefore reducing verification and data cost.
     *
     * @param hashes    Array of BN254 hashes.
     * @returns         validity proof with context
     */
    async getValidityProof(
        hashes: BN254[],
    ): Promise<CompressedProofWithContext> {
        /// get merkle proofs
        const merkleProofsWithContext =
            await this.getMultipleCompressedAccountProofs(hashes);

        /// to hex
        const inputs: HexInputsForProver = {
            roots: merkleProofsWithContext.map(ctx => toHex(bn(ctx.root))),
            inPathIndices: merkleProofsWithContext.map(
                proof => proof.leafIndex,
            ),
            inPathElements: merkleProofsWithContext.map(proof =>
                proof.merkleProof.map(proof => toHex(proof)),
            ),
            leaves: merkleProofsWithContext.map(proof => toHex(bn(proof.hash))),
        };

        const inputsData = JSON.stringify(inputs);

        const INCLUSION_PROOF_URL = `${this.proverEndpoint}/inclusion`;
        const response = await fetch(INCLUSION_PROOF_URL, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: inputsData,
        });
        if (!response.ok) {
            throw new Error(`Error fetching proof: ${response.statusText}`);
        }

        // TOOD: add type checks
        const data: any = await response.json();
        const parsed = proofFromJsonStruct(data);
        const compressedProof = negateAndCompressProof(parsed);

        const value: CompressedProofWithContext = {
            compressedProof,
            roots: merkleProofsWithContext.map(proof => proof.root),
            rootIndices: merkleProofsWithContext.map(proof => proof.rootIndex),
            leafIndices: merkleProofsWithContext.map(proof => proof.leafIndex),
            leaves: merkleProofsWithContext.map(proof => bn(proof.hash)),
            merkleTrees: merkleProofsWithContext.map(proof => proof.merkleTree),
            nullifierQueues: merkleProofsWithContext.map(
                proof => proof.nullifierQueue,
            ),
        };
        return value;
    }

    async getHealth(): Promise<string> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getHealth',
        );
        const res = create(unsafeRes, jsonRpcResult(HealthResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(res.error, 'failed to get health');
        }
        return res.result;
    }

    /** TODO: use from Connection */
    async getSlot(): Promise<number> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getSlot',
        );
        const res = create(unsafeRes, jsonRpcResult(SlotResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(res.error, 'failed to get slot');
        }
        return res.result;
    }

    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedTokenAccountsByOwner',
            { owner: owner.toBase58(), mint: options?.mint?.toBase58() },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(
                CompressedTokenAccountsByOwnerOrDelegateResult,
            ),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get info for compressed accounts owned by ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error('not implemented: NULL result');
        }
        const accounts: ParsedTokenAccount[] = [];
        /// TODO: clean up. Make typesafe
        res.result.value.items.map((item: any) => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    mockNullifierQueue,
                    item.hash.toArray(undefined, 32),
                    item.leafIndex,
                ),
                new PublicKey('9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE'), // TODO: photon should return programOwner
                bn(item.lamports),
                item.data && {
                    discriminator: item.discriminator.toArray('le', 8),
                    data: Buffer.from(item.data, 'base64'),
                    dataHash: item.dataHash.toArray('le', 32), //FIXME: need to calculate the hash or return from server
                },
                item.address,
            );

            const tokenData: TokenData = {
                mint: item.mint,
                owner: item.owner,
                amount: item.amount,
                delegate: item.delegate,
                state: 1, // TODO: dynamic
                isNative: null, // TODO: dynamic
                delegatedAmount: bn(0), // TODO: dynamic
            };

            accounts.push({
                compressedAccount: account,
                parsed: tokenData,
            });
        });

        /// TODO: consider custom sort. we're returning most recent first
        /// because thats how our tests expect it currently
        return accounts.sort(
            (a, b) =>
                b.compressedAccount.leafIndex - a.compressedAccount.leafIndex,
        );
    }

    /// TODO: implement delegate
    async getCompressedTokenAccountsByDelegate(
        _delegate: PublicKey,
        _options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
        throw new Error('Method not implemented.');
    }
    /// TODO: implement compressed token balance
    async getCompressedTokenAccountBalance(
        hash: BN254,
    ): Promise<{ amount: BN }> {
        throw new Error('Method not implemented.');
    }
}
