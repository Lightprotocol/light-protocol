import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    getCompressedAccountByHashTest,
    getCompressedAccountsByOwnerTest,
    getMultipleCompressedAccountsByHashTest,
    getQueueForTree,
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
    localTestActiveStateTreeInfo,
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
    CompressedAccountResultV2,
} from '../../rpc-interface';
import {
    CompressedProofWithContext,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ParsedTokenAccount,
    TokenBalance,
} from '../../rpc-interface';
import {
    BN254,
    CompressedAccountWithMerkleContext,
    CompressedProof,
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
import { StateTreeInfo } from '../../state/types';

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
 * Simple mock rpc for unit tests that simulates the compression rpc interface.
 * Fetches, parses events and builds merkletree on-demand, i.e. it does not persist state.
 * Constraints:
 * - Can only index 1 merkletree
 * - Can only index up to 1000 transactions
 *
 * For advanced testing use photon: https://github.com/helius-labs/photon
 */
export class TestRpc extends Connection implements CompressionApiInterface {
    compressionApiEndpoint: string;
    proverEndpoint: string;
    lightWasm: LightWasm;
    depth: number;
    log = false;
    activeStateTreeInfos: StateTreeInfo[] | null = null;

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
        super(endpoint, connectionConfig || 'confirmed');

        this.compressionApiEndpoint = compressionApiEndpoint;
        this.proverEndpoint = proverEndpoint;

        const { depth, log } = testRpcConfig ?? {};

        const { merkleTreeHeight } = defaultTestStateTreeAccounts();

        this.lightWasm = hasher;

        this.depth = depth ?? merkleTreeHeight;
        this.log = log ?? false;
    }

    /**
     * Manually set state tree addresses
     */
    setStateTreeInfo(info: StateTreeInfo[]): void {
        this.activeStateTreeInfos = info;
    }

    /**
     * Returns local test state trees.
     */
    async getCachedActiveStateTreeInfos(): Promise<StateTreeInfo[]> {
        return localTestActiveStateTreeInfo();
    }

    /**
     * Returns local test state trees.
     */
    async getLatestActiveStateTreeInfo(): Promise<StateTreeInfo[]> {
        return localTestActiveStateTreeInfo();
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
                treeType: TreeType;
                queue: PublicKey;
            }
        > = new Map();

        const cachedStateTreeInfos = await this.getCachedActiveStateTreeInfos();

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

                const { treeType, tree, queue } = getQueueForTree(
                    cachedStateTreeInfos,
                    treeOrQueue,
                );

                if (!leavesByTree.has(tree.toBase58())) {
                    leavesByTree.set(tree.toBase58(), {
                        leaves: [],
                        leafIndices: [],
                        treeType: treeType,
                        queue: queue,
                    });
                }

                const treeData = leavesByTree.get(tree.toBase58())!;
                treeData.leaves.push(hash);
                treeData.leafIndices.push(event.outputLeafIndices[index]);
            }
        }

        const merkleProofsMap: Map<string, MerkleContextWithMerkleProof> =
            new Map();

        for (const [
            treeKey,
            { leaves, treeType, queue },
        ] of leavesByTree.entries()) {
            const tree = new PublicKey(treeKey);

            let merkleTree: MerkleTree | undefined;
            if (treeType === TreeType.StateV1) {
                merkleTree = new MerkleTree(
                    this.depth,
                    this.lightWasm,
                    leaves.map(leaf => bn(leaf).toString()),
                );
            } else if (treeType === TreeType.StateV2) {
                /// In V2 State trees, The Merkle tree stays empty until the
                /// first forester transaction. And since test-rpc is only used
                /// for non-forested tests, we must return a tree with
                /// zerovalues.
                merkleTree = new MerkleTree(32, this.lightWasm, []);
            } else {
                throw new Error(
                    `Invalid tree type: ${treeType} in test-rpc.ts`,
                );
            }

            for (let i = 0; i < hashes.length; i++) {
                // const hashStr = hashes[i].toString();
                const leafIndex = leaves.findIndex(leaf =>
                    bn(leaf).eq(hashes[i]),
                );

                /// If leaf is part of current tree, return proof
                if (leafIndex !== -1) {
                    if (treeType === TreeType.StateV1) {
                        const pathElements =
                            merkleTree.path(leafIndex).pathElements;
                        const bnPathElements = pathElements.map(value =>
                            bn(value),
                        );
                        const root = bn(merkleTree.root());

                        const merkleProof: MerkleContextWithMerkleProof = {
                            hash: hashes[i].toArray('be', 32),
                            merkleTree: tree,
                            leafIndex: leafIndex,
                            merkleProof: bnPathElements,
                            queue: getQueueForTree(cachedStateTreeInfos, tree)
                                .queue,
                            rootIndex: leaves.length,
                            root: root,
                            treeType: treeType,
                            proveByIndex: true,
                        };

                        merkleProofsMap.set(hashes[i].toString(), merkleProof);
                    } else if (treeType === TreeType.StateV2) {
                        const pathElements = merkleTree._zeros.slice(0, -1);
                        const bnPathElements = pathElements.map(value =>
                            bn(value),
                        );
                        const root = bn(merkleTree.root());

                        const { tree: treeV2 } = getQueueForTree(
                            cachedStateTreeInfos,
                            tree,
                        );

                        /// get leafIndex from leavesByTree for the given hash
                        const leafIndex = leavesByTree
                            .get(treeV2.toBase58())!
                            .leafIndices.findIndex(index =>
                                hashes[i].eq(
                                    bn(
                                        leavesByTree.get(treeV2.toBase58())!
                                            .leaves[index],
                                    ),
                                ),
                            );

                        const merkleProof: MerkleContextWithMerkleProof = {
                            hash: new Array(32).fill(0),
                            merkleTree: treeV2,
                            leafIndex: leafIndex,
                            merkleProof: bnPathElements,
                            queue: PublicKey.default,
                            rootIndex: 0,
                            root,
                            // @ts-ignore
                            treeType: 0, // TODO: consider switching in photon
                            proveByIndex: true,
                        };

                        merkleProofsMap.set(hashes[i].toString(), merkleProof);
                    }
                }
            }
        }

        // Validate proofs
        merkleProofsMap.forEach((proof, index) => {
            const leafIndex = proof.leafIndex;
            const computedHash = leavesByTree.get(proof.merkleTree.toBase58())!
                .leaves[leafIndex];
            const hashArr = bn(computedHash).toArray('be', 32);
            if (
                !hashArr.every((val, index) => val === proof.hash[index]) &&
                proof.treeType === TreeType.StateV1 &&
                !proof.queue.equals(PublicKey.default)
            ) {
                throw new Error(
                    `Mismatch at index ${index}: expected ${proof.hash.toString()}, got ${hashArr.toString()}`,
                );
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
                return proof.treeType;
            }),
        );

        if (uniqueTreeTypes.size > 1) {
            throw new Error(
                'Requested hashes belong to different tree types (V1/V2)',
            );
        }

        // Return proofs in the order of requested hashes
        return hashes.map(hash => merkleProofsMap.get(hash.toString())!);
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
    ): Promise<CompressedTransaction | null> {
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
                merkleTree: defaultTestStateTreeAccounts().addressTree,
                queue: defaultTestStateTreeAccounts().addressQueue,
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
    ): Promise<WithContext<CompressedProofWithContext>> {
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        return {
            value: await this.getValidityProofV0(hashes, newAddresses),
            context: { slot: 1 },
        };
    }

    async getValidityProofV0(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<CompressedProofWithContext> {
        /// TODO(swen): add support for custom trees
        return this.getValidityProof(
            hashes.map(hash => hash.hash),
            newAddresses.map(address => address.address),
        );
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
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        let validityProof: CompressedProofWithContext;
        const infos = await this.getCachedActiveStateTreeInfos();

        if (hashes.length === 0 && newAddresses.length === 0) {
            throw new Error(
                'Empty input. Provide hashes and/or new addresses.',
            );
        } else if (hashes.length > 0 && newAddresses.length === 0) {
            /// inclusion

            const infoArray: {
                queue: PublicKey;
                treeType: TreeType;
                tree: PublicKey;
            }[] = [];

            for (const hash of hashes) {
                const proof = await this.getCompressedAccount(undefined, hash);
                const { queue, treeType, tree } = getQueueForTree(
                    infos,
                    proof!.merkleTree,
                );
                infoArray.push({ queue, treeType, tree });
            }

            const hasV1Accounts = infoArray.some(
                info => info.treeType === TreeType.StateV1,
            );

            let compressedProof: CompressedProof | null = null;
            if (infoArray.some(info => info.treeType === TreeType.StateV1)) {
                const merkleProofsWithContext =
                    await this.getMultipleCompressedAccountProofs(
                        hashes.filter(
                            (_, index) =>
                                infoArray[index].treeType === TreeType.StateV1,
                        ),
                    );
                const inputs = convertMerkleProofsWithContextToHex(
                    merkleProofsWithContext,
                );

                compressedProof = await proverRequest(
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
                    merkleTrees: merkleProofsWithContext.map(
                        proof => proof.merkleTree,
                    ),
                    queues: merkleProofsWithContext.map(proof => proof.queue),
                    proveByIndices: hashes.map(() => true),
                    treeTypes: merkleProofsWithContext.map(
                        () => TreeType.StateV1,
                    ),
                };
            } else {
                validityProof = {
                    compressedProof: null,
                    roots: [bn(0)],
                    rootIndices: hashes.map(_ => 0),
                    leafIndices: hashes.map(_ => 0), // TODO: check
                    leaves: hashes.map(hash => hash),
                    merkleTrees: hashes.map(
                        (_, index) => infoArray[index].tree,
                    ),
                    queues: hashes.map((_, index) => infoArray[index].queue),
                    proveByIndices: hashes.map(() => true),
                    treeTypes: hashes.map(() => TreeType.StateV2),
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
                merkleTrees: newAddressProofs.map(proof => proof.merkleTree),
                queues: newAddressProofs.map(proof => proof.queue),
                proveByIndices: newAddressProofs.map(_ => false),
                treeTypes: newAddressProofs.map(_ => TreeType.AddressV1),
            };
        } else if (hashes.length > 0 && newAddresses.length > 0) {
            /// combined
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            /// Test-RPC
            const infoArray: {
                queue: PublicKey;
                tree: PublicKey;
                treeType: TreeType;
            }[] = [];

            merkleProofsWithContext.forEach(proof => {
                const { queue, tree, treeType } = getQueueForTree(
                    infos,
                    proof.merkleTree,
                );
                infoArray.push({ queue, tree, treeType });
            });

            const hasV1Accounts = infoArray.some(
                info => info.treeType === TreeType.StateV1,
            );
            const hasV2Accounts = infoArray.some(
                info => info.treeType === TreeType.StateV2,
            );

            if (hasV1Accounts && hasV2Accounts) {
                throw new Error(
                    'Validity Proofs for mixed state trees (v1 and v2) are not supported.',
                );
            }
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );

            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);
            const newAddressInputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);
            const hasV1Addresses = newAddressProofs.some(_ => true); // All new address proofs are V1

            if (hasV1Addresses && hasV2Accounts) {
                throw new Error(
                    'Mixed V1 addresses and V2 accounts are not supported yet.',
                );
            }

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'combined',
                [inputs, newAddressInputs],
                this.log,
            );

            const treeTypes = [
                ...merkleProofsWithContext.map(() =>
                    hasV1Accounts ? TreeType.StateV1 : TreeType.StateV2,
                ),
                ...newAddressProofs.map(() => TreeType.AddressV1),
            ];

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
                            proof =>
                                proof.indexHashedIndexedElementLeaf.toNumber(), // TODO: support >32bit
                        ),
                    ),
                leaves: merkleProofsWithContext
                    .map(proof => bn(proof.hash))
                    .concat(newAddressProofs.map(proof => bn(proof.value))),
                merkleTrees: merkleProofsWithContext
                    .map(proof => proof.merkleTree)
                    .concat(newAddressProofs.map(proof => proof.merkleTree)),
                queues: merkleProofsWithContext
                    .map(proof => proof.queue)
                    .concat(newAddressProofs.map(proof => proof.queue)),
                proveByIndices: merkleProofsWithContext
                    .map(() => false)
                    .concat(newAddressProofs.map(() => false)),
                treeTypes,
            };
        } else throw new Error('Invalid input');

        return validityProof;
    }
}
