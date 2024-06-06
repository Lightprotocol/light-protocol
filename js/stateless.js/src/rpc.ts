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
    HexBatchInputsForProver,
    ValidityProofResult,
    NewAddressProofResult,
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
import { defaultTestStateTreeAccounts } from './constants';
import { BN } from '@coral-xyz/anchor';

import { toCamelCase, toHex } from './utils/conversion';

import {
    proofFromJsonStruct,
    negateAndCompressProof,
} from './utils/parse-validity-proof';

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
): Promise<ParsedTokenAccount[]> {
    const endpoint = filterByDelegate
        ? 'getCompressedTokenAccountsByDelegate'
        : 'getCompressedTokenAccountsByOwner';
    const propertyToCheck = filterByDelegate ? 'delegate' : 'owner';

    const unsafeRes = await rpcRequest(rpc.compressionApiEndpoint, endpoint, {
        [propertyToCheck]: ownerOrDelegate.toBase58(),
        mint: options.mint.toBase58(),
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

    res.result.value.items.map(item => {
        const _account = item.account;
        const _tokenData = item.tokenData;

        const compressedAccount: CompressedAccountWithMerkleContext =
            createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    _account.tree!,
                    mockNullifierQueue,
                    _account.hash.toArray(undefined, 32),
                    _account.leafIndex,
                ),
                new PublicKey('HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN'),
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
            isNative: _tokenData.isNative,
            delegatedAmount: _tokenData.delegatedAmount,
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
    return accounts.sort(
        (a, b) => b.compressedAccount.leafIndex - a.compressedAccount.leafIndex,
    );
}

/** @internal */
function buildCompressedAccountWithMaybeTokenData(account: any): {
    account: CompressedAccountWithMerkleContext;
    maybeTokenData: TokenData | null;
} {
    const tokenData = account.optionTokenData;
    const compressedAccount: CompressedAccountWithMerkleContext =
        createCompressedAccountWithMerkleContext(
            createMerkleContext(
                account.tree!,
                mockNullifierQueue,
                account.hash.toArray(undefined, 32),
                account.leafIndex,
            ),
            account.owner,
            bn(account.lamports),
            account.data ? parseAccountData(account.data) : undefined,
            account.address || undefined,
        );

    if (tokenData === null) {
        return { account: compressedAccount, maybeTokenData: null };
    }

    const parsed: TokenData = {
        mint: tokenData.mint,
        owner: tokenData.owner,
        amount: tokenData.amount,
        delegate: tokenData.delegate,
        state: ['uninitialized', 'initialized', 'frozen'].indexOf(
            tokenData.state,
        ),
        isNative: tokenData.isNative,
        delegatedAmount: tokenData.delegatedAmount,
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
    endpointOrWeb3JsConnection: string | Connection = 'http://127.0.0.1:8899',
    compressionApiEndpoint: string = 'http://127.0.0.1:8784',
    proverEndpoint: string = 'http://127.0.0.1:3001',
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
    params: any = [],
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

/** @internal */
export const proverRequest = async (
    proverEndpoint: string,
    method: 'inclusion' | 'new-address' | 'combined',
    params: any = [],
    log = false,
): Promise<CompressedProof> => {
    let logMsg: string = '';

    if (log) {
        logMsg = `Proof generation for method:${method}`;
        console.time(logMsg);
    }

    let body;
    if (method === 'inclusion') {
        body = JSON.stringify({ 'input-compressed-accounts': params });
    } else if (method === 'new-address') {
        body = JSON.stringify({ 'new-addresses': params });
    } else if (method === 'combined') {
        body = JSON.stringify({
            'input-compressed-accounts': params[0],
            'new-addresses': params[1],
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
    /// TODO: Move compression into the gnark prover to save bandwidth.
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
    leaf_index: BN;
    merkle_proof_hashed_indexed_element_leaf: BN[];
    index_hashed_indexed_element_leaf: BN;
};

export type MerkleContextWithNewAddressProof = {
    root: BN;
    value: BN;
    leafLowerRangeValue: BN;
    leafHigherRangeValue: BN;
    leafIndex: BN;
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
    leafIndex: number;
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
            leafIndex: nonInclusionMerkleProofInputs[i].leafIndex.toNumber(),
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

/// TODO: replace with dynamic nullifierQueue
const mockNullifierQueue = defaultTestStateTreeAccounts().nullifierQueue;

/**
 *
 */
export class Rpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;

    /**
     * Establish a Compression-compatible JSON RPC connection
     *
     * @param endpoint                      Endpoint to the solana cluster
     * @param compressionApiEndpoint        Endpoint to the compression server
     * @param proverEndpoint                Endpoint to the prover server.
     * @param connectionConfig              Optional connection config
     */
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
     * Fetch the compressed account for the specified account hash
     */
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
            item.data ? parseAccountData(item.data) : undefined,
            item.address || undefined,
        );
        return account;
    }

    /**
     * Fetch the compressed balance for the specified account hash
     */
    async getCompressedBalance(hash: BN254): Promise<BN> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedBalance',
            { hash: encodeBN254toBase58(hash) },
        );
        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(NativeBalanceResult),
        );
        if ('error' in res) {
            throw new SolanaJSONRPCError(
                res.error,
                `failed to get balance for compressed account ${hash.toString()}`,
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

        // const proofWithoutRoot = res.result.value.proof.slice(0, -1);

        // const root = res.result.value.proof[res.result.value.proof.length - 1];

        const value: MerkleContextWithMerkleProof = {
            hash: res.result.value.hash.toArray(undefined, 32),
            merkleTree: res.result.value.merkleTree,
            leafIndex: res.result.value.leafIndex,
            merkleProof: res.result.value.proof, //proofWithoutRoot,
            nullifierQueue: mockNullifierQueue, // TODO: use nullifierQueue from indexer
            rootIndex: res.result.value.rootSeq % 2400, // TODO: rootSeq % rootHistoryArray.length
            root: res.result.value.root, // TODO: validate correct root
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
        const accounts: CompressedAccountWithMerkleContext[] = [];
        res.result.value.items.map(item => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    mockNullifierQueue,
                    item.hash.toArray(undefined, 32),
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

        for (const proof of res.result.value) {
            const value: MerkleContextWithMerkleProof = {
                hash: proof.hash.toArray(undefined, 32),
                merkleTree: proof.merkleTree,
                leafIndex: proof.leafIndex,
                merkleProof: proof.proof,
                nullifierQueue: mockNullifierQueue, // TODO(photon): support nullifierQueue in response.
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

        res.result.value.items.map(item => {
            const account = createCompressedAccountWithMerkleContext(
                createMerkleContext(
                    item.tree!,
                    mockNullifierQueue,
                    item.hash.toArray(undefined, 32),
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
     * Fetch all the compressed token accounts owned by the specified public
     * key. Owner can be a program or user account
     */
    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
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
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
        return getCompressedTokenAccountsByOwnerOrDelegate(
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
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint
     */
    async getCompressedTokenBalancesByOwner(
        owner: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<{ balance: BN; mint: PublicKey }[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressedTokenBalancesByOwner',
            {
                owner: owner.toBase58(),
                mint: options.mint.toBase58(),
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

        /// filter by mint
        const filtered = res.result.value.tokenBalances.filter(
            tokenBalance =>
                tokenBalance.mint.toBase58() === options.mint.toBase58(),
        );

        return filtered;
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * account hash forward in time from genesis to the most recent confirmed
     * block
     *
     * @param hash queried account hash
     */
    async getSignaturesForCompressedAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForAccount', // TODO: update
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
        if (res.result.transaction === null) {
            console.log('getCompressedTransaction: returning null');
            return null;
        }

        const closedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[] = [];

        const openedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[] = [];

        res.result.compressionInfo.closedAccounts.map(item => {
            closedAccounts.push(buildCompressedAccountWithMaybeTokenData(item));
        });
        res.result.compressionInfo.openedAccounts.map(item => {
            openedAccounts.push(buildCompressedAccountWithMaybeTokenData(item));
        });

        return {
            compressionInfo: { closedAccounts, openedAccounts },
            transaction: res.result.transaction,
        };
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * address forward in time from genesis to the most recent confirmed
     * block
     *
     * @param address queried compressed account address
     */
    async getCompressionSignaturesForAddress(
        address: PublicKey,
    ): Promise<SignatureWithMetadata[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForAddress',
            { address: address.toBase58() },
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

        return res.result.value.items;
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
    ): Promise<SignatureWithMetadata[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForOwner',
            { owner: owner.toBase58() },
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

        return res.result.value.items;
    }

    /// TODO: needs mint
    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified token account owner forward in time from genesis to the most
     * recent confirmed block
     */
    async getCompressionSignaturesForTokenOwner(
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]> {
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getCompressionSignaturesForTokenOwner',
            { owner: owner.toBase58() },
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

        return res.result.value.items;
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
                value: proof.address,
                leafLowerRangeValue: proof.lowerRangeAddress,
                leafHigherRangeValue: proof.higherRangeAddress,
                leafIndex: bn(proof.leafIndex),
                merkleProofHashedIndexedElementLeaf: proof.proof,
                indexHashedIndexedElementLeaf: proof.lowElementLeafIndex,
                merkleTree: proof.merkleTree,
                nullifierQueue: mockNullifierQueue,
            };
            newAddressProofs.push(_proof);
        }
        return newAddressProofs;
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
     * @param hashes        Array of BN254 hashes.
     * @param newAddresses  Array of BN254 new addresses.
     * @returns             validity proof with context
     */
    async getValidityProof(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        let validityProof: CompressedProofWithContext;

        if (hashes.length === 0 && newAddresses.length === 0) {
            throw new Error(
                'Empty input. Provide hashes and/or new addresses.',
            );
        } else if (hashes.length > 0 && newAddresses.length === 0) {
            console.log(
                "NOTE: Manually calling prover for inclusion proof. To call Photon's 'getValidityProof' endpoint use 'getValidityProofDebug'.",
            );
            /// inclusion
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'inclusion',
                inputs,
                false,
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
            console.log(
                'NOTE: calling Photon getMultipleNewAddressProofs. May not exist yet.',
            );
            /// new-address
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const inputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'new-address',
                inputs,
                false,
            );

            validityProof = {
                compressedProof,
                roots: newAddressProofs.map(proof => proof.root),
                // TODO(crank): make dynamic to enable forester support in
                // test-rpc.ts. Currently this is a static root because the
                // address tree doesn't advance.
                rootIndices: newAddressProofs.map(_ => 3),
                leafIndices: newAddressProofs.map(
                    proof => proof.leafIndex.toNumber(), // TODO: support >32bit
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

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'combined',
                [inputs, newAddressInputs],
                false,
            );

            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext
                    .map(proof => proof.root)
                    .concat(newAddressProofs.map(proof => proof.root)),
                rootIndices: merkleProofsWithContext
                    .map(proof => proof.rootIndex)
                    // TODO(crank): make dynamic to enable forester support in
                    // test-rpc.ts. Currently this is a static root because the
                    // address tree doesn't advance.
                    .concat(newAddressProofs.map(_ => 3)),
                leafIndices: merkleProofsWithContext
                    .map(proof => proof.leafIndex)
                    .concat(
                        newAddressProofs.map(
                            proof => proof.leafIndex.toNumber(), // TODO: support >32bit
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
    // FIXME: debug photon zkp. For debugging use either
    // testRpc.getValidityProof or rpc.getValidityProof to test against
    async getValidityProofDebug(
        hashes: BN254[] = [],
        _newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        console.log("log: calling photon 'getValidityProof'");
        const unsafeRes = await rpcRequest(
            this.compressionApiEndpoint,
            'getValidityProof',
            hashes.map(hash => encodeBN254toBase58(hash)),
        );

        const res = create(unsafeRes, jsonRpcResult(ValidityProofResult));
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
        console.log(res.result);
        const value: CompressedProofWithContext = {
            compressedProof: res.result.compressedProof,
            merkleTrees: res.result.merkleTrees,
            leafIndices: res.result.leafIndices,
            nullifierQueues: res.result.merkleTrees.map(
                () => mockNullifierQueue,
            ), // TODO: use nullifierQueue from indexer
            rootIndices: res.result.rootIndices,
            roots: res.result.roots,
            leaves: res.result.leaves,
        };
        return value;
    }
}
