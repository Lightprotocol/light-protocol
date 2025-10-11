import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    getCompressedAccountByHashTest,
    getCompressedAccountsByOwnerTest,
    getMultipleCompressedAccountsByHashTest,
} from './get-compressed-accounts';
import {
    getCompressedTokenAccountByHashTest,
    getCompressedTokenAccountsByDelegateTest,
    getCompressedTokenAccountsByOwnerTest,
} from './get-compressed-token-accounts';
import { MerkleTree } from '../merkle-tree/merkle-tree';
import { getParsedEvents } from './get-parsed-events';
import {
    defaultTestStateTreeAccounts,
    localTestActiveStateTreeInfos,
    batchAddressTree,
} from '../../constants';
import {
    AddressWithTree,
    CompressedMintTokenHolders,
    CompressedTransaction,
    GetCompressedAccountsByOwnerConfig,
    PaginatedOptions,
    HashWithTree,
    LatestNonVotingSignatures,
    LatestNonVotingSignaturesPaginated,
    SignatureWithMetadata,
    WithContext,
    WithCursor,
} from '../../rpc-interface';
import {
    ValidityProofWithContext,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ParsedTokenAccount,
    TokenBalance,
} from '../../rpc-interface';
import {
    BN254,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    PublicTransactionEvent,
    TreeType,
    bn,
} from '../../state';
import { IndexedArray } from '../merkle-tree';
import {
    MerkleContextWithNewAddressProof,
    convertMerkleProofsWithContextToHex,
    convertNonInclusionMerkleProofInputsToHex,
    proverRequest,
} from '../../rpc';
import { TreeInfo } from '../../state/types';
import { getStateTreeInfoByPubkey } from '../../utils/get-state-tree-infos';

export interface TestRpcConfig {
    /**
     * Depth of state tree. Defaults to the public default test state tree depth
     */
    depth?: number;
    /**
     * Log proof generation time
     */
    log?: boolean;
}

export type ClientSubscriptionId = number;
export interface LightWasm {
    blakeHash(input: string | Uint8Array, hashLength: number): Uint8Array;
    poseidonHash(input: string[] | BN[]): Uint8Array;
    poseidonHashString(input: string[] | BN[]): string;
    poseidonHashBN(input: string[] | BN[]): BN;
}

/**
 * Returns a mock RPC instance for use in unit tests.
 *
 * @param lightWasm               Wasm hasher instance.
 * @param endpoint                RPC endpoint URL. Defaults to
 *                                'http://127.0.0.1:8899'.
 * @param proverEndpoint          Prover server endpoint URL. Defaults to
 *                                'http://localhost:3001'.
 * @param merkleTreeAddress       Address of the merkle tree to index. Defaults
 *                                to the public default test state tree.
 * @param nullifierQueueAddress   Optional address of the associated nullifier
 *                                queue.
 * @param depth                   Depth of the merkle tree.
 * @param log                     Log proof generation time.
 */
export async function getTestRpc(
    lightWasm: LightWasm,
    endpoint: string = 'http://127.0.0.1:8899',
    compressionApiEndpoint: string = 'http://127.0.0.1:8784',
    proverEndpoint: string = 'http://127.0.0.1:3001',
    depth?: number,
    log = false,
) {
    return new TestRpc(
        endpoint,
        lightWasm,
        compressionApiEndpoint,
        proverEndpoint,
        undefined,
        {
            depth: depth || defaultTestStateTreeAccounts().merkleTreeHeight,
            log,
        },
    );
}
/**
 * Mock RPC for unit tests that simulates the ZK Compression RPC interface.
 * Parses events and builds merkletree on-demand. It does not persist state.
 * Constraints:
 * - Can only index up to 1000 transactions
 *
 * For advanced testing use `Rpc` class which uses photon:
 * https://github.com/helius-labs/photon
 */
export class TestRpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;
    lightWasm: LightWasm;
    depth: number;
    log = false;
    allStateTreeInfos: TreeInfo[] | null = null;
    lastStateTreeFetchTime: number | null = null;
    fetchPromise: Promise<TreeInfo[]> | null = null;
    CACHE_TTL = 1000 * 60 * 60; // 1 hour

    /**
     * Establish a Compression-compatible JSON RPC mock-connection
     *
     * @param endpoint                  endpoint to the solana cluster (use for
     *                                  localnet only)
     * @param hasher                    light wasm hasher instance
     * @param compressionApiEndpoint    Endpoint to the compression server.
     * @param proverEndpoint            Endpoint to the prover server. defaults
     *                                  to endpoint
     * @param connectionConfig          Optional connection config
     * @param testRpcConfig             Config for the mock rpc
     */
    constructor(
        endpoint: string,
        hasher: LightWasm,
        compressionApiEndpoint: string,
        proverEndpoint: string,
        connectionConfig?: ConnectionConfig,
        testRpcConfig?: TestRpcConfig,
    ) {
        super(endpoint, connectionConfig || { commitment: 'confirmed' });

        this.compressionApiEndpoint = compressionApiEndpoint;
        this.proverEndpoint = proverEndpoint;

        const { depth, log } = testRpcConfig ?? {};
        const { merkleTreeHeight } = defaultTestStateTreeAccounts();

        this.lightWasm = hasher;
        this.depth = depth ?? merkleTreeHeight;
        this.log = log ?? false;
    }

    /**
     * @deprecated Use {@link getStateTreeInfos} instead
     */
    async getCachedActiveStateTreeInfo() {}
    /**
     * @deprecated Use {@link getStateTreeInfos} instead
     */
    async getCachedActiveStateTreeInfos() {}
    /**
     * Returns local test state trees.
     */
    async getStateTreeInfos(): Promise<TreeInfo[]> {
        return localTestActiveStateTreeInfos();
    }
    async doFetch(): Promise<TreeInfo[]> {
        throw new Error('doFetch not supported in test-rpc');
    }

    /**
     * Get a V2 address tree info.
     */
    async getAddressTreeInfoV2(): Promise<TreeInfo> {
        const tree = new PublicKey(batchAddressTree);
        return {
            tree,
            queue: tree,
            cpiContext: undefined,
            treeType: TreeType.AddressV2,
            nextTreeInfo: null,
        };
    }

    /**
     * Fetch the compressed account for the specified account hash
     */
    async getCompressedAccount(
        address?: BN254,
        hash?: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        if (address) {
            throw new Error('address is not supported in test-rpc');
        }
        if (!hash) {
            throw new Error('hash is required');
        }

        const account = await getCompressedAccountByHashTest(this, hash);
        return account ?? null;
    }

    /**
     * Fetch the compressed balance for the specified account hash
     */
    async getCompressedBalance(address?: BN254, hash?: BN254): Promise<BN> {
        if (address) {
            throw new Error('address is not supported in test-rpc');
        }
        if (!hash) {
            throw new Error('hash is required');
        }

        const account = await getCompressedAccountByHashTest(this, hash);
        if (!account) {
            throw new Error('Account not found');
        }
        return bn(account.lamports);
    }

    /**
     * Fetch the total compressed balance for the specified owner public key
     */
    async getCompressedBalanceByOwner(owner: PublicKey): Promise<BN> {
        const accounts = await this.getCompressedAccountsByOwner(owner);
        return accounts.items.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );
    }

    /**
     * Fetch the latest merkle proof for the specified account hash from the
     * cluster
     */
    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const proofs = await this.getMultipleCompressedAccountProofs([hash]);
        return proofs[0];
    }

    /**
     * Fetch all the account info for multiple compressed accounts specified by
     * an array of account hashes
     */
    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]> {
        return await getMultipleCompressedAccountsByHashTest(this, hashes);
    }
    /**
     * Ensure that the Compression Indexer has already indexed the transaction
     */
    async confirmTransactionIndexed(_slot: number): Promise<boolean> {
        return true;
    }

    /**
     * Fetch the latest merkle proofs for multiple compressed accounts specified
     * by an array account hashes
     */
    async getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]> {
        // Parse events and organize leaves by their respective merkle trees
        const events: PublicTransactionEvent[] = await getParsedEvents(
            this,
        ).then(events => events.reverse());
        const leavesByTree: Map<
            string,
            {
                leaves: number[][];
                leafIndices: number[];
                treeInfo: TreeInfo;
            }
        > = new Map();

        const cachedStateTreeInfos = await this.getStateTreeInfos();

        /// Assign leaves to their respective trees
        for (const event of events) {
            for (
                let index = 0;
                index < event.outputCompressedAccounts.length;
                index++
            ) {
                const hash = event.outputCompressedAccountHashes[index];
                const treeOrQueue =
                    event.pubkeyArray[
                        event.outputCompressedAccounts[index].merkleTreeIndex
                    ];

                const stateTreeInfo = getStateTreeInfoByPubkey(
                    cachedStateTreeInfos,
                    treeOrQueue,
                );

                if (!leavesByTree.has(stateTreeInfo.tree.toBase58())) {
                    leavesByTree.set(stateTreeInfo.tree.toBase58(), {
                        leaves: [],
                        leafIndices: [],
                        treeInfo: stateTreeInfo,
                    });
                }

                const treeData = leavesByTree.get(
                    stateTreeInfo.tree.toBase58(),
                );
                if (!treeData) {
                    throw new Error(
                        `Tree not found: ${stateTreeInfo.tree.toBase58()}`,
                    );
                }
                treeData.leaves.push(hash);
                treeData.leafIndices.push(event.outputLeafIndices[index]);
            }
        }

        const merkleProofsMap: Map<string, MerkleContextWithMerkleProof> =
            new Map();

        for (const [treeKey, { leaves, treeInfo }] of leavesByTree.entries()) {
            const tree = new PublicKey(treeKey);

            let merkleTree: MerkleTree | undefined;
            if (treeInfo.treeType === TreeType.StateV1) {
                merkleTree = new MerkleTree(
                    this.depth,
                    this.lightWasm,
                    leaves.map(leaf => bn(leaf).toString()),
                );
            } else if (treeInfo.treeType === TreeType.StateV2) {
                /// In V2 State trees, The Merkle tree stays empty until the
                /// first forester transaction. And since test-rpc is only used
                /// for non-forested tests, we must return a tree with
                /// zerovalues.
                merkleTree = new MerkleTree(32, this.lightWasm, []);
            } else {
                throw new Error(
                    `Invalid tree type: ${treeInfo.treeType} in test-rpc.ts`,
                );
            }

            for (let i = 0; i < hashes.length; i++) {
                const leafIndex = leaves.findIndex(leaf =>
                    bn(leaf).eq(hashes[i]),
                );

                /// If leaf is part of current tree, return proof
                if (leafIndex !== -1) {
                    if (treeInfo.treeType === TreeType.StateV1) {
                        const pathElements =
                            merkleTree.path(leafIndex).pathElements;
                        const bnPathElements = pathElements.map(value =>
                            bn(value),
                        );
                        const root = bn(merkleTree.root());

                        const merkleProof: MerkleContextWithMerkleProof = {
                            hash: bn(hashes[i].toArray('be', 32)),
                            treeInfo,
                            leafIndex,
                            merkleProof: bnPathElements,
                            proveByIndex: false,
                            rootIndex: leaves.length,
                            root,
                        };

                        merkleProofsMap.set(hashes[i].toString(), merkleProof);
                    } else if (treeInfo.treeType === TreeType.StateV2) {
                        const pathElements = merkleTree._zeros.slice(0, -1);
                        const bnPathElements = pathElements.map(value =>
                            bn(value),
                        );
                        const root = bn(merkleTree.root());

                        /// get leafIndex from leavesByTree for the given hash
                        const leafIndex = leavesByTree
                            .get(tree.toBase58())!
                            .leafIndices.findIndex(index =>
                                hashes[i].eq(
                                    bn(
                                        leavesByTree.get(tree.toBase58())!
                                            .leaves[index],
                                    ),
                                ),
                            );

                        const merkleProof: MerkleContextWithMerkleProof = {
                            // Hash is 0 for proveByIndex trees in test-rpc.
                            hash: bn(hashes[i].toArray('be', 32)),
                            // hash: bn(new Array(32).fill(0)),
                            treeInfo,
                            leafIndex,
                            merkleProof: bnPathElements,
                            proveByIndex: true,
                            // Root index is 0 for proveByIndex trees in
                            // test-rpc.
                            rootIndex: 0,
                            root,
                        };

                        merkleProofsMap.set(hashes[i].toString(), merkleProof);
                    }
                }
            }
        }

        // Validate proofs
        merkleProofsMap.forEach((proof, index) => {
            if (proof.treeInfo.treeType === TreeType.StateV1) {
                const leafIndex = proof.leafIndex;
                const computedHash = leavesByTree.get(
                    proof.treeInfo.tree.toBase58(),
                )!.leaves[leafIndex];
                const hashArr = bn(computedHash);
                if (!hashArr.eq(proof.hash)) {
                    throw new Error(
                        `Mismatch at index ${index}: expected ${proof.hash.toString()}, got ${hashArr.toString()}`,
                    );
                }
            }
        });

        // Ensure all requested hashes belong to the same tree type
        const uniqueTreeTypes = new Set(
            hashes.map(hash => {
                const proof = merkleProofsMap.get(hash.toString());
                if (!proof) {
                    throw new Error(
                        `Proof not found for hash: ${hash.toString()}`,
                    );
                }
                return proof.treeInfo.treeType;
            }),
        );

        if (uniqueTreeTypes.size > 1) {
            throw new Error(
                'Requested hashes belong to different tree types (V1/V2)',
            );
        }

        // Return proofs in the order of requested hashes
        return hashes.map(hash => {
            const proof = merkleProofsMap.get(hash.toString());
            if (!proof) {
                throw new Error(`No proof found for hash: ${hash.toString()}`);
            }
            return proof;
        });
    }
    /**
     * Fetch all the compressed accounts owned by the specified public key.
     * Owner can be a program or user account
     */
    async getCompressedAccountsByOwner(
        owner: PublicKey,
        _config?: GetCompressedAccountsByOwnerConfig,
    ): Promise<WithCursor<CompressedAccountWithMerkleContext[]>> {
        const accounts = await getCompressedAccountsByOwnerTest(this, owner);
        return {
            items: accounts,
            cursor: null,
        };
    }

    /**
     * Fetch the latest compression signatures on the cluster. Results are
     * paginated.
     */
    async getLatestCompressionSignatures(
        _cursor?: string,
        _limit?: number,
    ): Promise<LatestNonVotingSignaturesPaginated> {
        throw new Error(
            'getLatestNonVotingSignaturesWithContext not supported in test-rpc',
        );
    }
    /**
     * Fetch the latest non-voting signatures on the cluster. Results are
     * not paginated.
     */
    async getLatestNonVotingSignatures(
        _limit?: number,
    ): Promise<LatestNonVotingSignatures> {
        throw new Error(
            'getLatestNonVotingSignaturesWithContext not supported in test-rpc',
        );
    }
    /**
     * Fetch all the compressed token accounts owned by the specified public
     * key. Owner can be a program or user account
     */
    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        return await getCompressedTokenAccountsByOwnerTest(
            this,
            owner,
            options!.mint!,
        );
    }

    /**
     * Fetch all the compressed accounts delegated to the specified public key.
     */
    async getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        return await getCompressedTokenAccountsByDelegateTest(
            this,
            delegate,
            options.mint!,
        );
    }

    /**
     * Fetch the compressed token balance for the specified account hash
     */
    async getCompressedTokenAccountBalance(
        hash: BN254,
    ): Promise<{ amount: BN }> {
        const account = await getCompressedTokenAccountByHashTest(this, hash);
        return { amount: bn(account.parsed.amount) };
    }

    /**
     * @deprecated use {@link getCompressedTokenBalancesByOwnerV2}.
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint.
     */
    async getCompressedTokenBalancesByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<{ balance: BN; mint: PublicKey }[]>> {
        const accounts = await getCompressedTokenAccountsByOwnerTest(
            this,
            publicKey,
            options.mint!,
        );
        return {
            items: accounts.items.map(account => ({
                balance: bn(account.parsed.amount),
                mint: account.parsed.mint,
            })),
            cursor: null,
        };
    }

    /**
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint. Uses context.
     */
    async getCompressedTokenBalancesByOwnerV2(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithContext<WithCursor<TokenBalance[]>>> {
        const accounts = await getCompressedTokenAccountsByOwnerTest(
            this,
            publicKey,
            options.mint!,
        );
        return {
            context: { slot: 1 },
            value: {
                items: accounts.items.map(account => ({
                    balance: bn(account.parsed.amount),
                    mint: account.parsed.mint,
                })),
                cursor: null,
            },
        };
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * account hash forward in time from genesis to the most recent confirmed
     * block
     *
     * @param hash queried account hash
     */
    async getCompressionSignaturesForAccount(
        _hash: BN254,
    ): Promise<SignatureWithMetadata[]> {
        throw new Error(
            'getCompressionSignaturesForAccount not implemented in test-rpc',
        );
    }

    /**
     * Fetch a confirmed or finalized transaction from the cluster. Return with
     * CompressionInfo
     */
    async getTransactionWithCompressionInfo(
        _signature: string,
    ): Promise<CompressedTransaction> {
        throw new Error('getCompressedTransaction not implemented in test-rpc');
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * address forward in time from genesis to the most recent confirmed
     * block
     *
     * @param address queried compressed account address
     */
    async getCompressionSignaturesForAddress(
        _address: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForAddress3 not implemented');
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified account owner forward in time from genesis to the
     * most recent confirmed block
     *
     * @param owner queried owner public key
     */
    async getCompressionSignaturesForOwner(
        _owner: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForOwner not implemented');
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified token account owner forward in time from genesis to the most
     * recent confirmed block
     */
    async getCompressionSignaturesForTokenOwner(
        _owner: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForTokenOwner not implemented');
    }

    /**
     * Fetch the current indexer health status
     */
    async getIndexerHealth(): Promise<string> {
        return 'ok';
    }

    /**
     * Fetch the current slot that the node is processing
     */
    async getIndexerSlot(): Promise<number> {
        return 1;
    }

    /**
     * Fetch the latest address proofs for new unique addresses specified by an
     * array of addresses.
     *
     * the proof states that said address have not yet been created in respective address tree.
     * @param addresses Array of BN254 new addresses
     * @returns Array of validity proofs for new addresses
     */
    async getMultipleNewAddressProofs(addresses: BN254[]) {
        /// Build tree
        const indexedArray = IndexedArray.default();
        const allAddresses: BN[] = [];
        indexedArray.init();
        const hashes: BN[] = [];
        // TODO(crank): add support for cranked address tree in 'allAddresses'.
        // The Merkle tree root doesnt actually advance beyond init() unless we
        // start emptying the address queue.
        for (let i = 0; i < allAddresses.length; i++) {
            indexedArray.append(bn(allAddresses[i]));
        }
        for (let i = 0; i < indexedArray.elements.length; i++) {
            const hash = indexedArray.hashElement(this.lightWasm, i);
            hashes.push(bn(hash!));
        }
        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            hashes.map(hash => bn(hash).toString()),
        );

        /// Creates proof for each address
        const newAddressProofs: MerkleContextWithNewAddressProof[] = [];

        for (let i = 0; i < addresses.length; i++) {
            const [lowElement] = indexedArray.findLowElement(addresses[i]);
            if (!lowElement) throw new Error('Address not found');

            const leafIndex = lowElement.index;

            const pathElements: string[] = tree.path(leafIndex).pathElements;
            const bnPathElements = pathElements.map(value => bn(value));

            const higherRangeValue = indexedArray.get(
                lowElement.nextIndex,
            )!.value;
            const root = bn(tree.root());

            const proof: MerkleContextWithNewAddressProof = {
                root,
                rootIndex: 3,
                value: addresses[i],
                leafLowerRangeValue: lowElement.value,
                leafHigherRangeValue: higherRangeValue,
                nextIndex: bn(lowElement.nextIndex),
                merkleProofHashedIndexedElementLeaf: bnPathElements,
                indexHashedIndexedElementLeaf: bn(lowElement.index),
                treeInfo: {
                    tree: defaultTestStateTreeAccounts().addressTree,
                    queue: defaultTestStateTreeAccounts().addressQueue,
                    treeType: TreeType.AddressV1,
                    nextTreeInfo: null,
                },
            };
            newAddressProofs.push(proof);
        }
        return newAddressProofs;
    }

    async getCompressedMintTokenHolders(
        _mint: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithContext<WithCursor<CompressedMintTokenHolders[]>>> {
        throw new Error(
            'getCompressedMintTokenHolders not implemented in test-rpc',
        );
    }

    /**
     * @deprecated This method is not available for TestRpc. Please use
     * {@link getValidityProof} instead.
     */
    async getValidityProofAndRpcContext(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<WithContext<ValidityProofWithContext>> {
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        return {
            value: await this.getValidityProofV0(hashes, newAddresses),
            context: { slot: 1 },
        };
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
    ): Promise<ValidityProofWithContext> {
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        let validityProof: ValidityProofWithContext | null;

        const treeInfosUsed: TreeInfo[] = [];

        if (hashes.length === 0 && newAddresses.length === 0) {
            throw new Error(
                'Empty input. Provide hashes and/or new addresses.',
            );
        } else if (hashes.length > 0 && newAddresses.length === 0) {
            for (const hash of hashes) {
                const account = await this.getCompressedAccount(
                    undefined,
                    hash,
                );

                if (account) {
                    treeInfosUsed.push(account.treeInfo);
                } else throw new Error('Account not found');
            }
            const hasV1Accounts = treeInfosUsed.some(
                info => info.treeType === TreeType.StateV1,
            );

            /// inclusion
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            if (hasV1Accounts) {
                const inputs = convertMerkleProofsWithContextToHex(
                    merkleProofsWithContext,
                );

                const compressedProof = await proverRequest(
                    this.proverEndpoint,
                    'inclusion',
                    inputs,
                    this.log,
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
                    leaves: merkleProofsWithContext.map(proof =>
                        bn(proof.hash),
                    ),
                    treeInfos: merkleProofsWithContext.map(
                        proof => proof.treeInfo,
                    ),
                    proveByIndices: merkleProofsWithContext.map(
                        proof => proof.proveByIndex,
                    ),
                };
            } else {
                validityProof = {
                    compressedProof: null,
                    roots: merkleProofsWithContext.map(_proof => bn(0)),
                    rootIndices: merkleProofsWithContext.map(
                        proof => proof.rootIndex,
                    ),
                    leafIndices: merkleProofsWithContext.map(
                        proof => proof.leafIndex,
                    ),
                    leaves: merkleProofsWithContext.map(proof =>
                        bn(proof.hash),
                    ),
                    treeInfos: merkleProofsWithContext.map(
                        proof => proof.treeInfo,
                    ),
                    proveByIndices: merkleProofsWithContext.map(
                        proof => proof.proveByIndex,
                    ),
                };
            }
        } else if (hashes.length === 0 && newAddresses.length > 0) {
            /// new-address
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const inputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'new-address',
                inputs,
                this.log,
            );

            validityProof = {
                compressedProof,
                roots: newAddressProofs.map(proof => proof.root),
                rootIndices: newAddressProofs.map(_ => 3),
                leafIndices: newAddressProofs.map(proof =>
                    proof.indexHashedIndexedElementLeaf.toNumber(),
                ),
                leaves: newAddressProofs.map(proof => bn(proof.value)),
                treeInfos: newAddressProofs.map(proof => proof.treeInfo),
                proveByIndices: newAddressProofs.map(_ => false),
            };
        } else if (hashes.length > 0 && newAddresses.length > 0) {
            /// combined
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const treeInfosUsed = merkleProofsWithContext.map(
                proof => proof.treeInfo,
            );
            const hasV1Accounts = treeInfosUsed.some(
                info => info.treeType === TreeType.StateV1,
            );

            const newAddressInputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);

            let compressedProof;
            if (hasV1Accounts) {
                const inputs = convertMerkleProofsWithContextToHex(
                    merkleProofsWithContext,
                );

                compressedProof = await proverRequest(
                    this.proverEndpoint,
                    'combined',
                    [inputs, newAddressInputs],
                    true,
                );
            } else {
                // Still need to make the prover request for new addresses
                compressedProof = await proverRequest(
                    this.proverEndpoint,
                    'new-address',
                    newAddressInputs,
                    true,
                );
            }

            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext
                    .map(proof => (!hasV1Accounts ? bn(0) : proof.root)) // TODO: find better solution.
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
                        newAddressProofs.map(proof =>
                            proof.indexHashedIndexedElementLeaf.toNumber(),
                        ),
                    ),
                leaves: merkleProofsWithContext
                    .map(proof => bn(proof.hash))
                    .concat(newAddressProofs.map(proof => bn(proof.value))),
                treeInfos: merkleProofsWithContext
                    .map(proof => proof.treeInfo)
                    .concat(newAddressProofs.map(proof => proof.treeInfo)),
                proveByIndices: merkleProofsWithContext
                    .map(proof => proof.proveByIndex)
                    .concat(newAddressProofs.map(_ => false)),
            };
        } else throw new Error('Invalid input');

        return validityProof;
    }

    async getValidityProofV0(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<ValidityProofWithContext> {
        /// TODO(swen): add support for custom trees
        return this.getValidityProof(
            hashes.map(hash => hash.hash),
            newAddresses.map(address => address.address),
        );
    }
}
