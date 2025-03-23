import {
    Connection,
    ConnectionConfig,
    PublicKey,
    SolanaJSONRPCError,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    BalanceResult,
    CompressedAccountsByOwnerResultV2,
    CompressedProofWithContext,
    CompressedTokenAccountsByOwnerOrDelegateResultV2,
    CompressedTransaction,
    CompressedTransactionResultV2,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    HealthResult,
    HexInputsForProver,
    MultipleCompressedAccountsResultV2,
    NativeBalanceResult,
    ParsedTokenAccount,
    SignatureListResult,
    SignatureListWithCursorResult,
    SignatureWithMetadata,
    SlotResult,
    TokenBalanceListResult,
    jsonRpcResult,
    jsonRpcResultAndContext,
    ValidityProofResultV2,
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
    MerkleProofResultV2,
    CompressedAccountResultV2,
    MerkleContextV2Result,
    TokenDataResult,
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
    StateTreeInfo,
    TreeType,
    MerkleContext,
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
        ? 'getCompressedTokenAccountsByDelegateV2'
        : 'getCompressedTokenAccountsByOwnerV2';
    const propertyToCheck = filterByDelegate ? 'delegate' : 'owner';

    const unsafeRes = await rpcRequest(rpc.compressionApiEndpoint, endpoint, {
        [propertyToCheck]: ownerOrDelegate.toBase58(),
        mint: options.mint?.toBase58(),
        limit: options.limit?.toNumber(),
        cursor: options.cursor,
    });

    const res = create(
        unsafeRes,
        jsonRpcResultAndContext(
            CompressedTokenAccountsByOwnerOrDelegateResultV2,
        ),
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

    res.result.value.items.map(item => {
        const _account = item.account;
        const _tokenData = item.tokenData;

        const compressedAccount: CompressedAccountWithMerkleContext =
            createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    _account.merkleContext.tree,
                    _account.merkleContext.queue,
                    _account.hash.toArray('be', 32),
                    _account.leafIndex,
                    _account.merkleContext.treeType,
                    _account.proveByIndex,
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
    return {
        items: accounts.sort(
            (a, b) =>
                b.compressedAccount.leafIndex - a.compressedAccount.leafIndex,
        ),
        cursor: res.result.value.cursor,
    };
}

export interface NullifierMetadata {
    nullifier: BN254;
    txHash: BN254;
}

/** @internal */
function buildCompressedAccountWithMaybeTokenDataFromClosedAccountResultV2(
    closedAccountResultV2: any,
): {
    account: CompressedAccountWithMerkleContext;
    maybeTokenData: TokenData | null;
    maybeNullifierMetadata: NullifierMetadata | null;
} {
    const v1type = {
        account: closedAccountResultV2.account.account,
        optionalTokenData: closedAccountResultV2.optionalTokenData,
    };

    const v2NullifierMetadata = {
        nullifier: closedAccountResultV2.account.nullifier,
        txHash: closedAccountResultV2.account.txHash,
    };

    const x = buildCompressedAccountWithMaybeTokenData(v1type);
    const y = {
        account: x.account,
        maybeTokenData: x.maybeTokenData,
        maybeNullifierMetadata: v2NullifierMetadata,
    };
    return y;
}

/** @internal */
function buildCompressedAccountWithMaybeTokenData(
    accountStructWithOptionalTokenData: any,
): {
    account: CompressedAccountWithMerkleContext;
    maybeTokenData: TokenData | null;
} {
    const compressedAccountResult = accountStructWithOptionalTokenData.account;
    const tokenDataResult =
        accountStructWithOptionalTokenData.optionalTokenData;

    const compressedAccount: CompressedAccountWithMerkleContext =
        createCompressedAccountWithMerkleContext(
            createMerkleContext(
                compressedAccountResult.merkleTree,
                compressedAccountResult.queue,
                compressedAccountResult.hash.toArray('be', 32),
                compressedAccountResult.leafIndex,
                compressedAccountResult.treeType,
                compressedAccountResult.proveByIndex,
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
    queue: PublicKey;
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
 *
 */
export class Rpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;
    activeStateTreeInfos: StateTreeInfo[] | null = null;

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
        this.activeStateTreeInfos = info;
    }

    /**
     * Get the active state tree addresses from the cluster.
     * If not already cached, fetches from the cluster.
     */
    async getCachedActiveStateTreeInfos(): Promise<StateTreeInfo[]> {
        if (isLocalTest(this.rpcEndpoint)) {
            /// We don't have ALUTs on Localnet.
            return localTestActiveStateTreeInfo();
        }

        let info: StateTreeInfo[] | null = null;
        if (!this.activeStateTreeInfos) {
            const { mainnet, devnet } = defaultStateTreeLookupTables();
            try {
                info = await getActiveStateTreeInfos({
                    connection: this,
                    stateTreeLookupTableAddress:
                        mainnet[0].stateTreeLookupTable,
                    nullifyTableAddress: mainnet[0].nullifyTable,
                });
                this.activeStateTreeInfos = info;
            } catch {
                info = await getActiveStateTreeInfos({
                    connection: this,
                    stateTreeLookupTableAddress: devnet[0].stateTreeLookupTable,
                    nullifyTableAddress: devnet[0].nullifyTable,
                });
                this.activeStateTreeInfos = info;
            }
        }
        if (!this.activeStateTreeInfos) {
            throw new Error(
                `activeStateTreeInfos should not be null ${JSON.stringify(
                    this.activeStateTreeInfos,
                )}`,
            );
        }

        return this.activeStateTreeInfos!;
    }

    /**
     * Fetch the latest state tree addresses from the cluster.
     */
    async getLatestActiveStateTreeInfo(): Promise<StateTreeInfo[]> {
        this.activeStateTreeInfos = null;
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
            'getCompressedAccountV2',
            {
                hash: hash ? encodeBN254toBase58(hash) : undefined,
                address: address ? encodeBN254toBase58(address) : undefined,
            },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(nullable(CompressedAccountResultV2)),
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
        const item = res.result.value;

        const account = createCompressedAccountWithMerkleContext(
            createMerkleContext(
                item.merkleContext.tree,
                item.merkleContext.queue,
                item.hash.toArray('be', 32),
                item.leafIndex,
                item.merkleContext.treeType,
                item.proveByIndex,
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

    /**
     * Fetch the total compressed lamports balance for the specified owner
     * public key
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
     * @deprecated Almost always you want to use {@link getValidityProof}
     * instead.
     *
     * Note, Fetching merkle proofs for V2 accounts are not supported yet.
     *
     * Fetch the latest merkle proof for a compressed account specified by an
     * account hash
     */
    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedAccountProofV2',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(MerkleProofResultV2),
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

        const result = res.result.value;

        const treeContext: MerkleContext = {
            merkleTree: result.treeContext.tree,
            queue: result.treeContext.queue,
            hash: result.hash.toArray('be', 32),
            leafIndex: result.leafIndex,
            treeType: result.treeContext.treeType,
            proveByIndex: result.proveByIndex,
        };
        const value: MerkleContextWithMerkleProof = {
            merkleProof: result.proof,
            rootIndex: result.rootSeq % 2400,
            root: result.root,
            hash: treeContext.hash,
            merkleTree: treeContext.merkleTree,
            leafIndex: treeContext.leafIndex,
            queue: treeContext.queue,
            treeType: treeContext.treeType,
            proveByIndex: treeContext.proveByIndex,
        };
        return value;
    }

    /**
     * Fetch account infos for multiple compressed accounts specified by
     * an array of account hashes
     *
     * Returns sorted by most recent unspent account first.
     */
    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleCompressedAccountsV2',
            { hashes: hashes.map(hash => encodeBN254toBase58(hash)) },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(MultipleCompressedAccountsResultV2),
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
        const accounts: CompressedAccountWithMerkleContext[] = [];

        res.result.value.items.map(item => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.merkleContext.tree,
                    item.merkleContext.queue,
                    item.hash.toArray('be', 32),
                    item.leafIndex,
                    item.merkleContext.treeType,
                    item.proveByIndex,
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
     * @deprecated Almost always you want to use {@link getValidityProof} instead.
     *
     * Note, Fetching merkle proofs for V2 accounts are not supported yet.
     *
     * Fetch the latest merkle proofs for multiple compressed accounts specified
     * by an array account hashes
     */
    async getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getMultipleCompressedAccountProofsV2',
            hashes.map(hash => encodeBN254toBase58(hash)),
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(array(MerkleProofResultV2)),
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

        // const treeContexts: MerkleContext[] = [];
        for (const proof of res.result.value) {
            const ctx = {
                merkleTree: proof.treeContext.tree,
                queue: proof.treeContext.queue,
                hash: proof.hash.toArray('be', 32),
                leafIndex: proof.leafIndex,
                treeType: proof.treeContext.treeType,
                proveByIndex: proof.proveByIndex,
            };
            const value: MerkleContextWithMerkleProof = {
                hash: ctx.hash,
                merkleTree: ctx.merkleTree,
                leafIndex: ctx.leafIndex,
                merkleProof: proof.proof,
                queue: ctx.queue,
                rootIndex: proof.rootSeq % 2400,
                root: proof.root,
                treeType: ctx.treeType,
                proveByIndex: ctx.proveByIndex,
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
            'getCompressedAccountsByOwnerV2',
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
            jsonRpcResultAndContext(CompressedAccountsByOwnerResultV2),
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
        const { items } = res.result.value;

        const accounts: CompressedAccountWithMerkleContext[] = [];

        items.map(item => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.merkleContext.tree,
                    item.merkleContext.queue,
                    item.hash.toArray('be', 32),
                    item.leafIndex,
                    item.merkleContext.treeType,
                    item.proveByIndex,
                ),
                item.owner,
                bn(item.lamports),
                item.data ? parseAccountData(item.data) : undefined,
                item.address || undefined,
            );

            accounts.push(account);
        });

        const sorted = accounts.sort((a, b) => b.leafIndex - a.leafIndex);

        return {
            items: sorted,
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
            'getTransactionWithCompressionInfoV2',
            { signature },
        );

        const res = create(
            unsafeRes,
            jsonRpcResult(CompressedTransactionResultV2),
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

        res.result.compressionInfo.closedAccounts.map(item => {
            closedAccounts.push(
                buildCompressedAccountWithMaybeTokenDataFromClosedAccountResultV2(
                    item,
                ),
            );
        });
        res.result.compressionInfo.openedAccounts.map(item => {
            openedAccounts.push(buildCompressedAccountWithMaybeTokenData(item));
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
     * @deprecated Almost always you want to use {@link getValidityProofV0}
     *  instead.
     *
     * Fetch the latest address proofs for new unique addresses specified by an
     * array of addresses.
     *
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
                queue: defaultTestStateTreeAccounts().addressQueue,
            };
            newAddressProofs.push(_proof);
        }
        return newAddressProofs;
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
        const queues = accs.map(acc => acc.queue);

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
            'getValidityProofV2',
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
            jsonRpcResultAndContext(ValidityProofResultV2),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get ValidityProof for compressed accounts ${hashes.map(hash => hash.toString())}`,
            );
        }
        if (res.result === null) {
            throw new Error(
                `failed to get ValidityProof for compressed accounts ${hashes.map(hash => hash.toString())}`,
            );
        }

        const result = res.result.value;

        const proveByIndices = result.rootIndices.map(
            index => index.proveByIndex,
        );

        checkQueuesAndTreesMatchResponse({
            hashesWithTree: hashes,
            newAddresses,
            merkleContexts: result.merkleContexts,
        });

        checkVersionConsistency({
            hashes,
            treeTypes: result.merkleContexts.map(ctx => ctx.treeType),
            newAddresses,
        });

        const value: CompressedProofWithContext = {
            compressedProof: result.compressedProof,
            merkleTrees: result.merkleContexts.map(ctx => ctx.tree),
            leafIndices: result.leafIndices,
            queues: result.merkleContexts.map(ctx => ctx.queue),
            rootIndices: result.rootIndices.map(index => index.rootIndex),
            roots: result.roots,
            leaves: result.leaves,
            proveByIndices,
            treeTypes: result.merkleContexts.map(ctx => ctx.treeType),
        };
        return { value, context: res.result.context };
    }
}

/**
 * Helper function to validate the consistency of new addresses with their
 * corresponding Merkle contexts.
 *
 * @param {Array<HashWithTree>} hashesWithTree - Array of hashes with their
 * associated tree and queue.
 * @param {Array<AddressWithTree>} newAddresses - Array of new addresses with
 * their associated tree and queue.
 * @param {Array<MerkleContextV2>} merkleContexts - Array of Merkle contexts to
 * validate against.
 * @throws Will throw an error if there is a mismatch between the expected and
 * actual tree or queue.
 */
function checkQueuesAndTreesMatchResponse({
    hashesWithTree,
    newAddresses,
    merkleContexts,
}: {
    hashesWithTree: HashWithTree[];
    newAddresses: AddressWithTree[];
    merkleContexts: MerkleContextV2Result[];
}) {
    const merkleContextsState = merkleContexts.slice(0, hashesWithTree.length);
    const merkleContextsAddress = merkleContexts.slice(hashesWithTree.length);
    hashesWithTree.forEach((hashWithTree, index) => {
        const resTree = merkleContextsState[index].tree;
        const resQueue = merkleContextsState[index].queue;

        if (!hashWithTree.tree.equals(resTree)) {
            throw new Error(
                `Tree mismatch for hash ${encodeBN254toBase58(hashWithTree.hash)}: expected ${hashWithTree.tree.toBase58()}, got ${resTree.toBase58()}`,
            );
        }

        if (hashWithTree.queue && !hashWithTree.queue.equals(resQueue)) {
            throw new Error(
                `Queue mismatch for hash ${encodeBN254toBase58(hashWithTree.hash)}: expected ${hashWithTree.queue.toBase58()}, got ${resQueue ? resQueue.toBase58() : 'null'}`,
            );
        }
    });

    newAddresses.forEach((addressWithTree, index) => {
        const resTree = merkleContextsAddress[index].tree;
        const resQueue = merkleContextsAddress[index].queue;

        if (!addressWithTree.tree.equals(resTree)) {
            throw new Error(
                `Tree mismatch for address ${encodeBN254toBase58(addressWithTree.address)}: expected ${addressWithTree.tree.toBase58()}, got ${resTree.toBase58()}`,
            );
        }

        if (addressWithTree.queue && !addressWithTree.queue.equals(resQueue)) {
            throw new Error(
                `Queue mismatch for address ${encodeBN254toBase58(addressWithTree.address)}: expected ${addressWithTree.queue.toBase58()}, got ${resQueue ? resQueue.toBase58() : 'null'}`,
            );
        }
    });
}

/**
 * @internal
 */
export function checkVersionConsistency({
    hashes,
    treeTypes,
    newAddresses,
}: {
    hashes: HashWithTree[];
    treeTypes: TreeType[];
    newAddresses: AddressWithTree[];
}) {
    const stateTreeTypes = treeTypes.slice(0, hashes.length);
    const addressTreeTypes = treeTypes.slice(hashes.length);

    // Check if all hashes have the same version
    if (!stateTreeTypes.every(type => type === stateTreeTypes[0])) {
        throw new Error('Mixed V1 and V2 accounts are not supported');
    }

    // Check if all new addresses have the same version
    if (!addressTreeTypes.every(type => type === addressTreeTypes[0])) {
        throw new Error('Mixed V1 and V2 accounts are not supported');
    }

    // Ensure combined proofs are only with V1
    if (
        addressTreeTypes.length &&
        stateTreeTypes.some(type => type !== TreeType.StateV1)
    ) {
        throw new Error('Mixed V1 addresses and V2 accounts are not supported');
    }
}
