import {
    Connection,
    ConnectionConfig,
    PublicKey,
    SolanaJSONRPCError,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    BalanceResult,
    CompressedAccountResult,
    CompressedAccountsByOwnerResult,
    CompressedProofWithContext,
    CompressedTokenAccountsByOwnerOrDelegateResult,
    CompressedTransaction,
    CompressedTransactionResult,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    HealthResult,
    HexInputsForProver,
    MerkeProofResult,
    MultipleCompressedAccountsResult,
    NativeBalanceResult,
    ParsedTokenAccount,
    SignatureListResult,
    SignatureListWithCursorResult,
    SignatureWithMetadata,
    SlotResult,
    TokenBalanceListResult,
    jsonRpcResult,
    jsonRpcResultAndContext,
    ValidityProofResult,
    NewAddressProofResult,
    LatestNonVotingSignaturesResult,
    LatestNonVotingSignatures,
    LatestNonVotingSignaturesResultPaginated,
    LatestNonVotingSignaturesPaginated,
    WithContext,
    GetCompressedAccountsByOwnerConfig,
    WithCursor,
    AddressWithTree,
    HashWithTree,
    CompressedMintTokenHoldersResult,
    CompressedMintTokenHolders,
    TokenBalance,
    TokenBalanceListResultV2,
    PaginatedOptions,
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
    CompressedProof,
} from './state';
import { array, create, nullable } from 'superstruct';
import {
    defaultTestStateTreeAccounts,
    localTestActiveStateTreeInfo,
    isLocalTest,
    defaultStateTreeLookupTables,
} from './constants';
import BN from 'bn.js';
import { toCamelCase, toHex } from './utils/conversion';

import {
    proofFromJsonStruct,
    negateAndCompressProof,
} from './utils/parse-validity-proof';
import { LightWasm } from './test-helpers';
import { getActiveStateTreeInfos } from './utils/get-light-state-tree-info';
import { StateTreeInfo } from './state/types';
import { validateNumbersForProof } from './utils';

/** @internal */
export function parseAccountData({
    discriminator,
    data,
    dataHash,
}: {
    discriminator: BN;
    data: string;
    dataHash: BN;
}) {
    return {
        discriminator: discriminator.toArray('le', 8),
        data: Buffer.from(data, 'base64'),
        dataHash: dataHash.toArray('le', 32),
    };
}

/** @internal */
async function getCompressedTokenAccountsByOwnerOrDelegate(
    rpc: Rpc,
    ownerOrDelegate: PublicKey,
    options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    filterByDelegate: boolean = false,
): Promise<WithCursor<ParsedTokenAccount[]>> {
    const endpoint = filterByDelegate
        ? 'getCompressedTokenAccountsByDelegate'
        : 'getCompressedTokenAccountsByOwner';
    const propertyToCheck = filterByDelegate ? 'delegate' : 'owner';

    const unsafeRes = await rpcRequest(rpc.compressionApiEndpoint, endpoint, {
        [propertyToCheck]: ownerOrDelegate.toBase58(),
        mint: options.mint?.toBase58(),
        limit: options.limit?.toNumber(),
        cursor: options.cursor,
    });

    const res = create(
        unsafeRes,
        jsonRpcResultAndContext(CompressedTokenAccountsByOwnerOrDelegateResult),
    );
    if ('error' in res) {
        throw new SolanaJSONRPCError(
            res.error,
            `failed to get info for compressed accounts by ${propertyToCheck} ${ownerOrDelegate.toBase58()}`,
        );
    }
    if (res.result.value === null) {
        throw new Error('not implemented: NULL result');
    }
    const accounts: ParsedTokenAccount[] = [];

    const activeStateTreeInfo = await rpc.getCachedActiveStateTreeInfos();

    res.result.value.items.map(item => {
        const _account = item.account;
        const _tokenData = item.tokenData;

        const associatedQueue = getQueueForTree(
            activeStateTreeInfo,
            _account.tree!,
        );

        const compressedAccount: CompressedAccountWithMerkleContext =
            createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    _account.tree!,
                    associatedQueue,
                    _account.hash.toArray('be', 32),
                    _account.leafIndex,
                ),
                _account.owner,
                bn(_account.lamports),
                _account.data ? parseAccountData(_account.data) : undefined,
                _account.address || undefined,
            );

        const parsed: TokenData = {
            mint: _tokenData.mint,
            owner: _tokenData.owner,
            amount: _tokenData.amount,
            delegate: _tokenData.delegate,
            state: ['uninitialized', 'initialized', 'frozen'].indexOf(
                _tokenData.state,
            ),
            tlv: null,
        };

        if (
            parsed[propertyToCheck]?.toBase58() !== ownerOrDelegate.toBase58()
        ) {
            throw new Error(
                `RPC returned token account with ${propertyToCheck} different from requested ${propertyToCheck}`,
            );
        }

        accounts.push({
            compressedAccount,
            parsed,
        });
    });
    /// TODO: consider custom or different sort. Most recent here.
    return {
        items: accounts.sort(
            (a, b) =>
                b.compressedAccount.leafIndex - a.compressedAccount.leafIndex,
        ),
        cursor: res.result.value.cursor,
    };
}

/** @internal */
function buildCompressedAccountWithMaybeTokenData(
    accountStructWithOptionalTokenData: any,
    activeStateTreeInfo: StateTreeInfo[],
): {
    account: CompressedAccountWithMerkleContext;
    maybeTokenData: TokenData | null;
} {
    const compressedAccountResult = accountStructWithOptionalTokenData.account;
    const tokenDataResult =
        accountStructWithOptionalTokenData.optionalTokenData;

    const associatedQueue = getQueueForTree(
        activeStateTreeInfo,
        compressedAccountResult.tree!,
    );
    const compressedAccount: CompressedAccountWithMerkleContext =
        createCompressedAccountWithMerkleContext(
            createMerkleContext(
                compressedAccountResult.merkleTree,
                associatedQueue,
                compressedAccountResult.hash.toArray('be', 32),
                compressedAccountResult.leafIndex,
            ),
            compressedAccountResult.owner,
            bn(compressedAccountResult.lamports),
            compressedAccountResult.data
                ? parseAccountData(compressedAccountResult.data)
                : undefined,
            compressedAccountResult.address || undefined,
        );

    if (tokenDataResult === null) {
        return { account: compressedAccount, maybeTokenData: null };
    }

    const parsed: TokenData = {
        mint: tokenDataResult.mint,
        owner: tokenDataResult.owner,
        amount: tokenDataResult.amount,
        delegate: tokenDataResult.delegate,
        state: ['uninitialized', 'initialized', 'frozen'].indexOf(
            tokenDataResult.state,
        ),
        tlv: null,
    };

    return { account: compressedAccount, maybeTokenData: parsed };
}

/**
 * Establish a Compression-compatible JSON RPC connection
 *
 * @param endpointOrWeb3JsConnection    endpoint to the solana cluster or
 *                                      Connection object
 * @param compressionApiEndpoint        Endpoint to the compression server
 * @param proverEndpoint                Endpoint to the prover server. defaults
 *                                      to endpoint
 * @param connectionConfig              Optional connection config
 */
export function createRpc(
    endpointOrWeb3JsConnection?: string | Connection,
    compressionApiEndpoint?: string,
    proverEndpoint?: string,
    config?: ConnectionConfig,
): Rpc {
    const localEndpoint = 'http://127.0.0.1:8899';
    const localCompressionApiEndpoint = 'http://127.0.0.1:8784';
    const localProverEndpoint = 'http://127.0.0.1:3001';

    let endpoint: string;

    if (!endpointOrWeb3JsConnection) {
        // Local as default
        endpoint = localEndpoint;
        compressionApiEndpoint =
            compressionApiEndpoint || localCompressionApiEndpoint;
        proverEndpoint = proverEndpoint || localProverEndpoint;
    } else if (typeof endpointOrWeb3JsConnection === 'string') {
        endpoint = endpointOrWeb3JsConnection;
        compressionApiEndpoint = compressionApiEndpoint || endpoint;
        proverEndpoint = proverEndpoint || endpoint;
    } else if (endpointOrWeb3JsConnection instanceof Connection) {
        endpoint = endpointOrWeb3JsConnection.rpcEndpoint;
        compressionApiEndpoint = compressionApiEndpoint || endpoint;
        proverEndpoint = proverEndpoint || endpoint;
    }
    // 3
    else {
        throw new Error('Invalid endpoint or connection type');
    }

    return new Rpc(endpoint, compressionApiEndpoint, proverEndpoint, config);
}

/**
 * Helper function to preprocess the response to wrap numbers as strings
 * @param {string} text - The JSON string to preprocess
 * @returns {string} - The preprocessed JSON string with numbers wrapped as strings
 */
export function wrapBigNumbersAsStrings(text: string): string {
    return text.replace(/(":\s*)(-?\d+)(\s*[},])/g, (match, p1, p2, p3) => {
        const num = Number(p2);
        if (
            !Number.isNaN(num) &&
            (num > Number.MAX_SAFE_INTEGER || num < Number.MIN_SAFE_INTEGER)
        ) {
            return `${p1}"${p2}"${p3}`;
        }
        return match;
    });
}

/** @internal */
export const rpcRequest = async (
    rpcEndpoint: string,
    method: string,
    params: any = [],
    convertToCamelCase = true,
    debug = false,
): Promise<any> => {
    const body = JSON.stringify({
        jsonrpc: '2.0',
        id: 'test-account',
        method: method,
        params: params,
    });

    if (debug) {
        const generateCurlSnippet = () => {
            const escapedBody = body.replace(/"/g, '\\"');
            return `curl -X POST ${rpcEndpoint} \\
     -H "Content-Type: application/json" \\
     -d "${escapedBody}"`;
        };

        console.log('Debug: Stack trace:');
        console.log(new Error().stack);
        console.log('\nDebug: curl:');
        console.log(generateCurlSnippet());
        console.log('\n');
    }

    const response = await fetch(rpcEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: body,
    });

    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }

    const text = await response.text();

    const wrappedJsonString = wrapBigNumbersAsStrings(text);

    if (convertToCamelCase) {
        return toCamelCase(JSON.parse(wrappedJsonString));
    }

    return JSON.parse(wrappedJsonString);
};

/** @internal */
export const proverRequest = async (
    proverEndpoint: string,
    method: 'inclusion' | 'new-address' | 'combined',
    params: any = [],
    log = false,
    publicInputHash: BN | undefined = undefined,
): Promise<CompressedProof> => {
    let logMsg: string = '';

    if (log) {
        logMsg = `Proof generation for method:${method}`;
        console.time(logMsg);
    }

    let body;
    if (method === 'inclusion') {
        body = JSON.stringify({
            circuitType: 'inclusion',
            stateTreeHeight: 26,
            inputCompressedAccounts: params,
            // publicInputHash: publicInputHash.toString('hex'),
        });
    } else if (method === 'new-address') {
        body = JSON.stringify({
            circuitType: 'non-inclusion',
            addressTreeHeight: 26,
            // publicInputHash: publicInputHash.toString('hex'),
            newAddresses: params,
        });
    } else if (method === 'combined') {
        body = JSON.stringify({
            circuitType: 'combined',
            // publicInputHash: publicInputHash.toString('hex'),
            stateTreeHeight: 26,
            addressTreeHeight: 26,
            inputCompressedAccounts: params[0],
            newAddresses: params[1],
        });
    }

    const response = await fetch(`${proverEndpoint}/prove`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: body,
    });

    if (!response.ok) {
        throw new Error(`Error fetching proof: ${response.statusText}`);
    }
    const data: any = await response.json();
    const parsed = proofFromJsonStruct(data);
    const compressedProof = negateAndCompressProof(parsed);

    if (log) console.timeEnd(logMsg);

    return compressedProof;
};

export type NonInclusionMerkleProofInputs = {
    root: BN;
    value: BN;
    leaf_lower_range_value: BN;
    leaf_higher_range_value: BN;
    nextIndex: BN;
    merkle_proof_hashed_indexed_element_leaf: BN[];
    index_hashed_indexed_element_leaf: BN;
};

export type MerkleContextWithNewAddressProof = {
    root: BN;
    rootIndex: number;
    value: BN;
    leafLowerRangeValue: BN;
    leafHigherRangeValue: BN;
    nextIndex: BN;
    merkleProofHashedIndexedElementLeaf: BN[];
    indexHashedIndexedElementLeaf: BN;
    merkleTree: PublicKey;
    nullifierQueue: PublicKey;
};

export type NonInclusionJsonStruct = {
    root: string;
    value: string;
    pathIndex: number;
    pathElements: string[];
    leafLowerRangeValue: string;
    leafHigherRangeValue: string;
    nextIndex: number;
};

export function convertMerkleProofsWithContextToHex(
    merkleProofsWithContext: MerkleContextWithMerkleProof[],
): HexInputsForProver[] {
    const inputs: HexInputsForProver[] = [];

    for (let i = 0; i < merkleProofsWithContext.length; i++) {
        const input: HexInputsForProver = {
            root: toHex(merkleProofsWithContext[i].root),
            pathIndex: merkleProofsWithContext[i].leafIndex,
            pathElements: merkleProofsWithContext[i].merkleProof.map(hex =>
                toHex(hex),
            ),
            leaf: toHex(bn(merkleProofsWithContext[i].hash)),
        };
        inputs.push(input);
    }

    return inputs;
}

export function convertNonInclusionMerkleProofInputsToHex(
    nonInclusionMerkleProofInputs: MerkleContextWithNewAddressProof[],
): NonInclusionJsonStruct[] {
    const inputs: NonInclusionJsonStruct[] = [];
    for (let i = 0; i < nonInclusionMerkleProofInputs.length; i++) {
        const input: NonInclusionJsonStruct = {
            root: toHex(nonInclusionMerkleProofInputs[i].root),
            value: toHex(nonInclusionMerkleProofInputs[i].value),
            pathIndex:
                nonInclusionMerkleProofInputs[
                    i
                ].indexHashedIndexedElementLeaf.toNumber(),
            pathElements: nonInclusionMerkleProofInputs[
                i
            ].merkleProofHashedIndexedElementLeaf.map(hex => toHex(hex)),
            nextIndex: nonInclusionMerkleProofInputs[i].nextIndex.toNumber(),
            leafLowerRangeValue: toHex(
                nonInclusionMerkleProofInputs[i].leafLowerRangeValue,
            ),
            leafHigherRangeValue: toHex(
                nonInclusionMerkleProofInputs[i].leafHigherRangeValue,
            ),
        };
        inputs.push(input);
    }
    return inputs;
}

function calculateTwoInputsHashChain(
    hashesFirst: BN[],
    hashesSecond: BN[],
    lightWasm: LightWasm,
): BN {
    if (hashesFirst.length !== hashesSecond.length) {
        throw new Error('Input lengths must match.');
    }
    if (hashesFirst.length === 0) {
        return new BN(0);
    }

    let hashChain = lightWasm.poseidonHashBN([
        hashesFirst[0].toString(),
        hashesSecond[0].toString(),
    ]);

    for (let i = 1; i < hashesFirst.length; i++) {
        hashChain = lightWasm.poseidonHashBN([
            hashChain.toString(),
            hashesFirst[i].toString(),
            hashesSecond[i].toString(),
        ]);
    }

    return hashChain;
}

export function getPublicInputHash(
    accountProofs: MerkleContextWithMerkleProof[],
    accountHashes: BN254[],
    newAddressProofs: MerkleContextWithNewAddressProof[],
    lightWasm: LightWasm,
): BN {
    const accountRoots = accountProofs.map(x => x.root);
    const inclusionHashChain = calculateTwoInputsHashChain(
        accountRoots,
        accountHashes,
        lightWasm,
    );

    const newAddressHashes = newAddressProofs.map(x => x.value);
    const newAddressRoots = newAddressProofs.map(x => x.root);
    const nonInclusionHashChain = calculateTwoInputsHashChain(
        newAddressRoots,
        newAddressHashes,
        lightWasm,
    );

    if (!nonInclusionHashChain.isZero()) {
        return nonInclusionHashChain;
    } else if (!inclusionHashChain.isZero()) {
        return inclusionHashChain;
    } else {
        return calculateTwoInputsHashChain(
            [inclusionHashChain],
            [nonInclusionHashChain],
            lightWasm,
        );
    }
}

/**
 * Get the queue for a given tree
 *
 * @param info - The active state tree addresses
 * @param tree - The tree to get the queue for
 * @returns The queue for the given tree, or undefined if not found
 */
export function getQueueForTree(
    info: StateTreeInfo[],
    tree: PublicKey,
): PublicKey {
    const index = info.findIndex(t => t.tree.equals(tree));
    if (index === -1) {
        throw new Error(
            'No associated queue found for tree. Please set activeStateTreeInfo with latest Tree accounts. If you use custom state trees, set manually.',
        );
    }
    if (!info[index].queue) {
        throw new Error('Queue must not be null for state tree');
    }
    return info[index].queue;
}

/**
 * Get the tree for a given queue
 *
 * @param info - The active state tree addresses
 * @param queue - The queue to get the tree for
 * @returns The tree for the given queue, or undefined if not found
 */
export function getTreeForQueue(
    info: StateTreeInfo[],
    queue: PublicKey,
): PublicKey {
    const index = info.findIndex(q => q.queue?.equals(queue));
    if (index === -1) {
        throw new Error(
            'No associated tree found for queue. Please set activeStateTreeInfo with latest Tree accounts. If you use custom state trees, set manually.',
        );
    }
    if (!info[index].tree) {
        throw new Error('Tree must not be null for state tree');
    }
    return info[index].tree;
}

/**
 *
 */
export class Rpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;
    activeStateTreeInfo: StateTreeInfo[] | null = null;

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

    /**
     * Manually set state tree addresses
     */
    setStateTreeInfo(info: StateTreeInfo[]): void {
        this.activeStateTreeInfo = info;
    }

    /**
     *
     * Get the active state tree addresses from the cluster.
     * If not already cached, fetches from the cluster.
     */
    async getCachedActiveStateTreeInfos(): Promise<StateTreeInfo[]> {
        if (isLocalTest(this.rpcEndpoint)) {
            return localTestActiveStateTreeInfo();
        }

        let info: StateTreeInfo[] | null = null;
        if (!this.activeStateTreeInfo) {
            const { mainnet, devnet } = defaultStateTreeLookupTables();
            try {
                info = await getActiveStateTreeInfos({
                    connection: this,
                    stateTreeLookupTableAddress:
                        mainnet[0].stateTreeLookupTable,
                    nullifyTableAddress: mainnet[0].nullifyTable,
                });
                this.activeStateTreeInfo = info;
            } catch {
                info = await getActiveStateTreeInfos({
                    connection: this,
                    stateTreeLookupTableAddress: devnet[0].stateTreeLookupTable,
                    nullifyTableAddress: devnet[0].nullifyTable,
                });
                this.activeStateTreeInfo = info;
            }
        }
        if (!this.activeStateTreeInfo) {
            throw new Error(
                `activeStateTreeInfo should not be null ${JSON.stringify(
                    this.activeStateTreeInfo,
                )}`,
            );
        }

        return this.activeStateTreeInfo!;
    }

    /**
     * Fetch the latest state tree addresses from the cluster.
     */
    async getActiveStateTreeInfos(): Promise<StateTreeInfo[]> {
        this.activeStateTreeInfo = null;
        return await this.getCachedActiveStateTreeInfos();
    }

    /**
     * Fetch the compressed account for the specified account address or hash
     */
    async getCompressedAccount(
        address?: BN254,
        hash?: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        if (!hash && !address) {
            throw new Error('Either hash or address must be provided');
        }
        if (hash && address) {
            throw new Error('Only one of hash or address must be provided');
        }
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccount',
            {
                hash: hash ? encodeBN254toBase58(hash) : undefined,
                address: address ? encodeBN254toBase58(address) : undefined,
            },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(nullable(CompressedAccountResult)),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get info for compressed account ${hash ? hash.toString() : address ? address.toString() : ''}`,
            );
        }
        if (res.result.value === null) {
            return null;
        }

        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();
        const associatedQueue = getQueueForTree(
            activeStateTreeInfo,
            res.result.value.tree!,
        );
        const item = res.result.value;
        const account = createCompressedAccountWithMerkleContext(
            createMerkleContext(
                item.tree!,
                associatedQueue,
                item.hash.toArray('be', 32),
                item.leafIndex,
            ),
            item.owner,
            bn(item.lamports),
            item.data ? parseAccountData(item.data) : undefined,
            item.address || undefined,
        );
        return account;
    }

    /**
     * Fetch the compressed balance for the specified account address or hash
     */
    async getCompressedBalance(address?: BN254, hash?: BN254): Promise<BN> {
        if (!hash && !address) {
            throw new Error('Either hash or address must be provided');
        }
        if (hash && address) {
            throw new Error('Only one of hash or address must be provided');
        }
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedBalance',
            {
                hash: hash ? encodeBN254toBase58(hash) : undefined,
                address: address ? encodeBN254toBase58(address) : undefined,
            },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(NativeBalanceResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get balance for compressed account ${hash ? hash.toString() : address ? address.toString() : ''}`,
            );
        }
        if (res.result.value === null) {
            return bn(0);
        }

        return bn(res.result.value);
    }

    /// TODO: validate that this is just for sol accounts
    /**
     * Fetch the total compressed balance for the specified owner public key
     */
    async getCompressedBalanceByOwner(owner: PublicKey): Promise<BN> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedBalanceByOwner',
            { owner: owner.toBase58() },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(NativeBalanceResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get balance for compressed account ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            return bn(0);
        }
        return bn(res.result.value);
    }

    /**
     * Fetch the latest merkle proof for the specified account hash from the
     * cluster
     */
    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccountProof',
            { hash: encodeBN254toBase58(hash) },
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
        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();
        const associatedQueue = getQueueForTree(
            activeStateTreeInfo,
            res.result.value.merkleTree,
        );

        const value: MerkleContextWithMerkleProof = {
            hash: res.result.value.hash.toArray('be', 32),
            merkleTree: res.result.value.merkleTree,
            leafIndex: res.result.value.leafIndex,
            merkleProof: res.result.value.proof,
            nullifierQueue: associatedQueue, // TODO(photon): support nullifierQueue in response.
            rootIndex: res.result.value.rootSeq % 2400,
            root: res.result.value.root,
        };
        return value;
    }

    /**
     * Fetch all the account info for multiple compressed accounts specified by
     * an array of account hashes
     */
    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleCompressedAccounts',
            { hashes: hashes.map(hash => encodeBN254toBase58(hash)) },
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
            throw new Error(
                `failed to get info for compressed accounts ${hashes.map(hash => encodeBN254toBase58(hash)).join(', ')}`,
            );
        }
        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();
        const accounts: CompressedAccountWithMerkleContext[] = [];
        res.result.value.items.map(item => {
            const associatedQueue = getQueueForTree(
                activeStateTreeInfo,
                item.tree!,
            );
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    associatedQueue,
                    item.hash.toArray('be', 32),
                    item.leafIndex,
                ),
                item.owner,
                bn(item.lamports),
                item.data ? parseAccountData(item.data) : undefined,
                item.address || undefined,
            );
            accounts.push(account);
        });

        return accounts.sort((a, b) => b.leafIndex - a.leafIndex);
    }

    /**
     * Fetch the latest merkle proofs for multiple compressed accounts specified
     * by an array account hashes
     */
    async getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]> {
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

        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();
        for (const proof of res.result.value) {
            const associatedQueue = getQueueForTree(
                activeStateTreeInfo,
                proof.merkleTree,
            );
            const value: MerkleContextWithMerkleProof = {
                hash: proof.hash.toArray('be', 32),
                merkleTree: proof.merkleTree,
                leafIndex: proof.leafIndex,
                merkleProof: proof.proof,
                nullifierQueue: associatedQueue,
                rootIndex: proof.rootSeq % 2400,
                root: proof.root,
            };
            merkleProofs.push(value);
        }
        return merkleProofs;
    }

    /**
     * Fetch all the compressed accounts owned by the specified public key.
     * Owner can be a program or user account
     */
    async getCompressedAccountsByOwner(
        owner: PublicKey,
        config?: GetCompressedAccountsByOwnerConfig | undefined,
    ): Promise<WithCursor<CompressedAccountWithMerkleContext[]>> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccountsByOwner',
            {
                owner: owner.toBase58(),
                filters: config?.filters || [],
                dataSlice: config?.dataSlice,
                cursor: config?.cursor,
                limit: config?.limit?.toNumber(),
            },
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
            return {
                items: [],
                cursor: null,
            };
        }
        const accounts: CompressedAccountWithMerkleContext[] = [];
        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();

        res.result.value.items.map(item => {
            const associatedQueue = getQueueForTree(
                activeStateTreeInfo,
                item.tree!,
            );
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    associatedQueue,
                    item.hash.toArray('be', 32),
                    item.leafIndex,
                ),
                item.owner,
                bn(item.lamports),
                item.data ? parseAccountData(item.data) : undefined,
                item.address || undefined,
            );

            accounts.push(account);
        });

        return {
            items: accounts.sort((a, b) => b.leafIndex - a.leafIndex),
            cursor: res.result.value.cursor,
        };
    }

    /**
     * Fetch all the compressed token accounts owned by the specified public
     * key. Owner can be a program or user account
     */
    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        if (!options) options = {};

        return await getCompressedTokenAccountsByOwnerOrDelegate(
            this,
            owner,
            options,
            false,
        );
    }

    /**
     * Fetch all the compressed accounts delegated to the specified public key.
     */
    async getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        if (!options) options = {};

        return await getCompressedTokenAccountsByOwnerOrDelegate(
            this,
            delegate,
            options,
            true,
        );
    }

    /**
     * Fetch the compressed token balance for the specified account hash
     */
    async getCompressedTokenAccountBalance(
        hash: BN254,
    ): Promise<{ amount: BN }> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedTokenAccountBalance',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(unsafeRes, jsonRpcResultAndContext(BalanceResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get balance for compressed token account ${hash.toString()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get balance for compressed token account ${hash.toString()}`,
            );
        }

        return { amount: bn(res.result.value.amount) };
    }

    /**
     * @deprecated use {@link getCompressedTokenBalancesByOwnerV2} instead.
     *
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint. Returns without context.
     */
    async getCompressedTokenBalancesByOwner(
        owner: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<TokenBalance[]>> {
        if (!options) options = {};

        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedTokenBalancesByOwner',
            {
                owner: owner.toBase58(),
                mint: options.mint?.toBase58(),
                limit: options.limit?.toNumber(),
                cursor: options.cursor,
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(TokenBalanceListResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get compressed token balances for owner ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get compressed token balances for owner ${owner.toBase58()}`,
            );
        }

        const maybeFiltered = options.mint
            ? res.result.value.tokenBalances.filter(
                  tokenBalance =>
                      tokenBalance.mint.toBase58() === options.mint!.toBase58(),
              )
            : res.result.value.tokenBalances;

        return {
            items: maybeFiltered,
            cursor: res.result.value.cursor,
        };
    }

    /**
     * Fetch the compressed token balances owned by the specified public
     * key. Paginated. Can filter by mint. Returns with context.
     */
    async getCompressedTokenBalancesByOwnerV2(
        owner: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithContext<WithCursor<TokenBalance[]>>> {
        if (!options) options = {};

        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedTokenBalancesByOwnerV2',
            {
                owner: owner.toBase58(),
                mint: options.mint?.toBase58(),
                limit: options.limit?.toNumber(),
                cursor: options.cursor,
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(TokenBalanceListResultV2),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get compressed token balances for owner ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get compressed token balances for owner ${owner.toBase58()}`,
            );
        }

        const maybeFiltered = options.mint
            ? res.result.value.items.filter(
                  tokenBalance =>
                      tokenBalance.mint.toBase58() === options.mint!.toBase58(),
              )
            : res.result.value.items;

        return {
            context: res.result.context,
            value: {
                items: maybeFiltered,
                cursor: res.result.value.cursor,
            },
        };
    }

    /**
     * Returns confirmed compression signatures for transactions involving the specified
     * account hash forward in time from genesis to the most recent confirmed
     * block
     *
     * @param hash queried account hash
     */
    async getCompressionSignaturesForAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForAccount',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(SignatureListResult),
        );

        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get signatures for compressed account ${hash.toString()}`,
            );
        }
        return res.result.value.items;
    }

    /**
     * Fetch a confirmed or finalized transaction from the cluster. Return with
     * CompressionInfo
     */
    async getTransactionWithCompressionInfo(
        signature: string,
    ): Promise<CompressedTransaction | null> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getTransactionWithCompressionInfo',
            { signature },
        );

        const res = create(
            unsafeRes,
            jsonRpcResult(CompressedTransactionResult),
        );

        if ('error' in res) {
            throw new SolanaJSONRPCError(res.error, 'failed to get slot');
        }

        if (res.result.transaction === null) return null;

        const closedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[] = [];

        const openedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[] = [];

        const activeStateTreeInfo = await this.getCachedActiveStateTreeInfos();

        res.result.compressionInfo.closedAccounts.map(item => {
            closedAccounts.push(
                buildCompressedAccountWithMaybeTokenData(
                    item,
                    activeStateTreeInfo,
                ),
            );
        });
        res.result.compressionInfo.openedAccounts.map(item => {
            openedAccounts.push(
                buildCompressedAccountWithMaybeTokenData(
                    item,
                    activeStateTreeInfo,
                ),
            );
        });

        const calculateTokenBalances = (
            accounts: Array<{
                account: CompressedAccountWithMerkleContext;
                maybeTokenData: TokenData | null;
            }>,
        ):
            | Array<{
                  owner: PublicKey;
                  mint: PublicKey;
                  amount: BN;
              }>
            | undefined => {
            const balances = Object.values(
                accounts.reduce(
                    (acc, { maybeTokenData }) => {
                        if (maybeTokenData) {
                            const { owner, mint, amount } = maybeTokenData;
                            const key = `${owner.toBase58()}_${mint.toBase58()}`;
                            if (key in acc) {
                                acc[key].amount = acc[key].amount.add(amount);
                            } else {
                                acc[key] = { owner, mint, amount };
                            }
                        }
                        return acc;
                    },
                    {} as {
                        [key: string]: {
                            owner: PublicKey;
                            mint: PublicKey;
                            amount: BN;
                        };
                    },
                ),
            );
            return balances.length > 0 ? balances : undefined;
        };

        const preTokenBalances = calculateTokenBalances(closedAccounts);
        const postTokenBalances = calculateTokenBalances(openedAccounts);

        return {
            compressionInfo: {
                closedAccounts,
                openedAccounts,
                preTokenBalances,
                postTokenBalances,
            },
            transaction: res.result.transaction,
        };
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * address forward in time from genesis to the most recent confirmed block
     *
     * @param address queried compressed account address
     */
    async getCompressionSignaturesForAddress(
        address: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForAddress',
            {
                address: address.toBase58(),
                cursor: options?.cursor,
                limit: options?.limit?.toNumber(),
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(SignatureListWithCursorResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get signatures for address ${address.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get signatures for address ${address.toBase58()}`,
            );
        }

        return res.result.value;
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified account owner forward in time from genesis to the
     * most recent confirmed block
     *
     * @param owner queried owner public key
     */
    async getCompressionSignaturesForOwner(
        owner: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForOwner',
            {
                owner: owner.toBase58(),
                cursor: options?.cursor,
                limit: options?.limit?.toNumber(),
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(SignatureListWithCursorResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get signatures for owner ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get signatures for owner ${owner.toBase58()}`,
            );
        }

        return res.result.value;
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified token account owner forward in time from genesis to the most
     * recent confirmed block
     */
    async getCompressionSignaturesForTokenOwner(
        owner: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForTokenOwner',
            {
                owner: owner.toBase58(),
                cursor: options?.cursor,
                limit: options?.limit?.toNumber(),
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(SignatureListWithCursorResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get signatures for owner ${owner.toBase58()}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get signatures for owner ${owner.toBase58()}`,
            );
        }

        return res.result.value;
    }

    /**
     * Fetch the current indexer health status
     */
    async getIndexerHealth(): Promise<string> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getIndexerHealth',
        );
        const res = create(unsafeRes, jsonRpcResult(HealthResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(res.error, 'failed to get health');
        }
        return res.result;
    }

    /**
     * Ensure that the Compression Indexer has already indexed the transaction
     */
    async confirmTransactionIndexed(slot: number): Promise<boolean> {
        const startTime = Date.now();
        // eslint-disable-next-line no-constant-condition
        while (true) {
            const indexerSlot = await this.getIndexerSlot();

            if (indexerSlot >= slot) {
                return true;
            }
            if (Date.now() - startTime > 20000) {
                // 20 seconds
                throw new Error(
                    'Timeout: Indexer slot did not reach the required slot within 20 seconds',
                );
            }
            await new Promise(resolve => setTimeout(resolve, 200));
        }
    }

    /**
     * Fetch the current slot that the node is processing
     */
    async getIndexerSlot(): Promise<number> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getIndexerSlot',
        );
        const res = create(unsafeRes, jsonRpcResult(SlotResult));
        if ('error' in res) {
            throw new SolanaJSONRPCError(res.error, 'failed to get slot');
        }
        return res.result;
    }

    /**
     * Fetch all the compressed token holders for a given mint. Paginated.
     */
    async getCompressedMintTokenHolders(
        mint: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithContext<WithCursor<CompressedMintTokenHolders[]>>> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedMintTokenHolders',
            {
                mint: mint.toBase58(),
                cursor: options?.cursor,
                limit: options?.limit?.toNumber(),
            },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(CompressedMintTokenHoldersResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                'failed to get mint token holders',
            );
        }

        return res.result;
    }
    /**
     * Fetch the latest compression signatures on the cluster. Results are
     * paginated.
     */
    async getLatestCompressionSignatures(
        cursor?: string,
        limit?: number,
    ): Promise<LatestNonVotingSignaturesPaginated> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getLatestCompressionSignatures',
            { limit, cursor },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(LatestNonVotingSignaturesResultPaginated),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                'failed to get latest non-voting signatures',
            );
        }
        return res.result;
    }

    /**
     * Fetch all non-voting signatures
     */
    async getLatestNonVotingSignatures(
        limit?: number,
        cursor?: string,
    ): Promise<LatestNonVotingSignatures> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getLatestNonVotingSignatures',
            { limit, cursor },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(LatestNonVotingSignaturesResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                'failed to get latest non-voting signatures',
            );
        }
        return res.result;
    }

    /**
     * Fetch the latest address proofs for new unique addresses specified by an
     * array of addresses.
     *
     * the proof states that said address have not yet been created in
     * respective address tree.
     * @param addresses Array of BN254 new addresses
     * @returns Array of validity proofs for new addresses
     */
    async getMultipleNewAddressProofs(addresses: BN254[]) {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleNewAddressProofs',
            addresses.map(address => encodeBN254toBase58(address)),
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(array(NewAddressProofResult)),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get proofs for new addresses ${addresses.map(address => encodeBN254toBase58(address)).join(', ')}`,
            );
        }
        if (res.result.value === null) {
            throw new Error(
                `failed to get proofs for new addresses ${addresses.map(address => encodeBN254toBase58(address)).join(', ')}`,
            );
        }

        /// Creates proof for each address
        const newAddressProofs: MerkleContextWithNewAddressProof[] = [];

        for (const proof of res.result.value) {
            const _proof: MerkleContextWithNewAddressProof = {
                root: proof.root,
                rootIndex: proof.rootSeq % 2400,
                value: proof.address,
                leafLowerRangeValue: proof.lowerRangeAddress,
                leafHigherRangeValue: proof.higherRangeAddress,
                nextIndex: bn(proof.nextIndex),
                merkleProofHashedIndexedElementLeaf: proof.proof,
                indexHashedIndexedElementLeaf: bn(proof.lowElementLeafIndex),
                merkleTree: proof.merkleTree,
                nullifierQueue: defaultTestStateTreeAccounts().addressQueue,
            };
            newAddressProofs.push(_proof);
        }
        return newAddressProofs;
    }

    /**
     * Advanced usage of getValidityProof: fetches ZKP directly from a custom
     * non-rpcprover. Note: This uses the proverEndpoint specified in the
     * constructor. For normal usage, please use {@link getValidityProof}
     * instead.
     *
     * Fetch the latest validity proof for (1) compressed accounts specified by
     * an array of account hashes. (2) new unique addresses specified by an
     * array of addresses.
     *
     * Validity proofs prove the presence of compressed accounts in state trees
     * and the non-existence of addresses in address trees, respectively. They
     * enable verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes        Array of BN254 hashes.
     * @param newAddresses  Array of BN254 new addresses.
     * @returns             validity proof with context
     */
    async getValidityProofDirect(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        let validityProof: CompressedProofWithContext;

        if (hashes.length === 0 && newAddresses.length === 0) {
            throw new Error(
                'Empty input. Provide hashes and/or new addresses.',
            );
        } else if (hashes.length > 0 && newAddresses.length === 0) {
            /// inclusion
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );
            // const lightWasm = await WasmFactory.getInstance();
            // const publicInputHash = getPublicInputHash(
            //     merkleProofsWithContext,
            //     hashes,
            //     [],
            //     lightWasm,
            // );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'inclusion',
                inputs,
                false,
                // publicInputHash,
            );
            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext.map(proof => proof.root),
                rootIndices: merkleProofsWithContext.map(
                    proof => proof.rootIndex,
                ),
                leafIndices: merkleProofsWithContext.map(
                    proof => proof.leafIndex,
                ),
                leaves: merkleProofsWithContext.map(proof => bn(proof.hash)),
                merkleTrees: merkleProofsWithContext.map(
                    proof => proof.merkleTree,
                ),
                nullifierQueues: merkleProofsWithContext.map(
                    proof => proof.nullifierQueue,
                ),
            };
        } else if (hashes.length === 0 && newAddresses.length > 0) {
            /// new-address
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const inputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);
            // const lightWasm = await WasmFactory.getInstance();
            // const publicInputHash = getPublicInputHash(
            //     [],
            //     [],
            //     newAddressProofs,
            //     lightWasm,
            // );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'new-address',
                inputs,
                false,
                // publicInputHash,
            );

            validityProof = {
                compressedProof,
                roots: newAddressProofs.map(proof => proof.root),
                rootIndices: newAddressProofs.map(proof => proof.rootIndex),
                leafIndices: newAddressProofs.map(proof =>
                    proof.nextIndex.toNumber(),
                ),
                leaves: newAddressProofs.map(proof => bn(proof.value)),
                merkleTrees: newAddressProofs.map(proof => proof.merkleTree),
                nullifierQueues: newAddressProofs.map(
                    proof => proof.nullifierQueue,
                ),
            };
        } else if (hashes.length > 0 && newAddresses.length > 0) {
            /// combined
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const newAddressInputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);
            // const lightWasm = await WasmFactory.getInstance();
            // const publicInputHash = getPublicInputHash(
            //     merkleProofsWithContext,
            //     hashes,
            //     newAddressProofs,
            //     lightWasm,
            // );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'combined',
                [inputs, newAddressInputs],
                false,
                // publicInputHash,
            );

            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext
                    .map(proof => proof.root)
                    .concat(newAddressProofs.map(proof => proof.root)),
                rootIndices: merkleProofsWithContext
                    .map(proof => proof.rootIndex)
                    .concat(newAddressProofs.map(proof => proof.rootIndex)),
                leafIndices: merkleProofsWithContext
                    .map(proof => proof.leafIndex)
                    .concat(
                        newAddressProofs.map(
                            proof => proof.nextIndex.toNumber(), // TODO: support >32bit
                        ),
                    ),
                leaves: merkleProofsWithContext
                    .map(proof => bn(proof.hash))
                    .concat(newAddressProofs.map(proof => bn(proof.value))),
                merkleTrees: merkleProofsWithContext
                    .map(proof => proof.merkleTree)
                    .concat(newAddressProofs.map(proof => proof.merkleTree)),
                nullifierQueues: merkleProofsWithContext
                    .map(proof => proof.nullifierQueue)
                    .concat(
                        newAddressProofs.map(proof => proof.nullifierQueue),
                    ),
            };
        } else throw new Error('Invalid input');

        return validityProof;
    }

    /**
     * @deprecated use {@link getValidityProofV0} instead.
     *
     *
     *
     * Fetch the latest validity proof for (1) compressed accounts specified by
     * an array of account hashes. (2) new unique addresses specified by an
     * array of addresses.
     *
     * Validity proofs prove the presence of compressed accounts in state trees
     * and the non-existence of addresses in address trees, respectively. They
     * enable verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes        Array of BN254 hashes.
     * @param newAddresses  Array of BN254 new addresses.
     * @returns             validity proof with context
     */
    async getValidityProof(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        const accs = await this.getMultipleCompressedAccounts(hashes);
        const trees = accs.map(acc => acc.merkleTree);
        const queues = accs.map(acc => acc.nullifierQueue);

        // TODO: add dynamic address tree support here
        const defaultAddressTreePublicKey =
            defaultTestStateTreeAccounts().addressTree;
        const defaultAddressQueuePublicKey =
            defaultTestStateTreeAccounts().addressQueue;

        const formattedHashes = hashes.map((item, index) => {
            return {
                hash: item,
                tree: trees[index],
                queue: queues[index],
            };
        });

        const formattedNewAddresses = newAddresses.map(item => {
            return {
                address: item,
                tree: defaultAddressTreePublicKey,
                queue: defaultAddressQueuePublicKey,
            };
        });

        return this.getValidityProofV0(formattedHashes, formattedNewAddresses);
    }

    /**
     * Fetch the latest validity proof for (1) compressed accounts specified by
     * an array of account hashes. (2) new unique addresses specified by an
     * array of addresses.
     *
     * Validity proofs prove the presence of compressed accounts in state trees
     * and the non-existence of addresses in address trees, respectively. They
     * enable verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes        Array of { hash: BN254, tree: PublicKey, queue: PublicKey }.
     * @param newAddresses  Array of { address: BN254, tree: PublicKey, queue: PublicKey }.
     * @returns             validity proof with context
     */
    async getValidityProofV0(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<CompressedProofWithContext> {
        const { value } = await this.getValidityProofAndRpcContext(
            hashes,
            newAddresses,
        );
        return value;
    }

    /**
     * Fetch the latest validity proof for (1) compressed accounts specified by
     * an array of account hashes. (2) new unique addresses specified by an
     * array of addresses. Returns with context slot.
     *
     * Validity proofs prove the presence of compressed accounts in state trees
     * and the non-existence of addresses in address trees, respectively. They
     * enable verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes        Array of BN254 hashes.
     * @param newAddresses  Array of BN254 new addresses. Optionally specify the
     *                      tree and queue for each address. Default to public
     *                      state tree/queue.
     * @returns             validity proof with context
     */
    async getValidityProofAndRpcContext(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<WithContext<CompressedProofWithContext>> {
        validateNumbersForProof(hashes.length, newAddresses.length);

        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getValidityProof',
            {
                hashes: hashes.map(({ hash }) => encodeBN254toBase58(hash)),
                newAddressesWithTrees: newAddresses.map(
                    ({ address, tree }) => ({
                        address: encodeBN254toBase58(address),
                        tree: tree.toBase58(),
                    }),
                ),
            },
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(ValidityProofResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get ValidityProof for compressed accounts ${hashes.map(hash => hash.toString())}`,
            );
        }

        const result = res.result.value;

        if (result === null) {
            throw new Error(
                `failed to get ValidityProof for compressed accounts ${hashes.map(hash => hash.toString())}`,
            );
        }

        const value: CompressedProofWithContext = {
            compressedProof: result.compressedProof,
            merkleTrees: result.merkleTrees,
            leafIndices: result.leafIndices,
            nullifierQueues: [
                ...hashes.map(({ queue }) => queue),
                ...newAddresses.map(({ queue }) => queue),
            ],
            rootIndices: result.rootIndices,
            roots: result.roots,
            leaves: result.leaves,
        };
        return { value, context: res.result.context };
    }
}
