import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
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
import { defaultTestStateTreeAccounts } from '../../constants';
import {
    CompressedTransaction,
    LatestNonVotingSignatures,
    LatestNonVotingSignaturesPaginated,
    SignatureWithMetadata,
    WithContext,
} from '../../rpc-interface';
import {
    CompressedProofWithContext,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ParsedTokenAccount,
} from '../../rpc-interface';
import {
    BN254,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    PublicTransactionEvent,
    bn,
} from '../../state';
import { IndexedArray } from '../merkle-tree';
import {
    MerkleContextWithNewAddressProof,
    convertMerkleProofsWithContextToHex,
    convertNonInclusionMerkleProofInputsToHex,
    proverRequest,
} from '../../rpc';

export interface TestRpcConfig {
    /**
     * Address of the state tree to index. Default: public default test state
     * tree.
     */
    merkleTreeAddress?: PublicKey;
    /**
     * Nullifier queue associated with merkleTreeAddress
     */
    nullifierQueueAddress?: PublicKey;
    /**
     * Depth of state tree. Defaults to the public default test state tree depth
     */
    depth?: number;
    /**
     * Log proof generation time
     */
    log?: boolean;
    /**
     * Address of the address tree to index. Default: public default test
     * address tree.
     */
    addressTreeAddress?: PublicKey;
    /**
     * Address queue associated with addressTreeAddress
     */
    addressQueueAddress?: PublicKey;
}

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
    merkleTreeAddress?: PublicKey,
    nullifierQueueAddress?: PublicKey,
    depth?: number,
    log = false,
) {
    const defaultAccounts = defaultTestStateTreeAccounts();

    return new TestRpc(
        endpoint,
        lightWasm,
        compressionApiEndpoint,
        proverEndpoint,
        undefined,
        {
            merkleTreeAddress: merkleTreeAddress || defaultAccounts.merkleTree,
            nullifierQueueAddress:
                nullifierQueueAddress || defaultAccounts.nullifierQueue,
            depth: depth || defaultAccounts.merkleTreeHeight,
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
    merkleTreeAddress: PublicKey;
    nullifierQueueAddress: PublicKey;
    addressTreeAddress: PublicKey;
    addressQueueAddress: PublicKey;
    lightWasm: LightWasm;
    depth: number;
    log = false;

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

        const {
            merkleTreeAddress,
            nullifierQueueAddress,
            depth,
            log,
            addressTreeAddress,
            addressQueueAddress,
        } = testRpcConfig ?? {};

        const {
            merkleTree,
            nullifierQueue,
            merkleTreeHeight,
            addressQueue,
            addressTree,
        } = defaultTestStateTreeAccounts();

        this.lightWasm = hasher;
        this.merkleTreeAddress = merkleTreeAddress ?? merkleTree;
        this.nullifierQueueAddress = nullifierQueueAddress ?? nullifierQueue;
        this.addressTreeAddress = addressTreeAddress ?? addressTree;
        this.addressQueueAddress = addressQueueAddress ?? addressQueue;
        this.depth = depth ?? merkleTreeHeight;
        this.log = log ?? false;
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
        return accounts.reduce(
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
        /// Build tree
        const events: PublicTransactionEvent[] = await getParsedEvents(
            this,
        ).then(events => events.reverse());
        const allLeaves: number[][] = [];
        const allLeafIndices: number[] = [];
        for (const event of events) {
            for (
                let index = 0;
                index < event.outputCompressedAccounts.length;
                index++
            ) {
                const hash = event.outputCompressedAccountHashes[index];

                allLeaves.push(hash);
                allLeafIndices.push(event.outputLeafIndices[index]);
            }
        }
        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            allLeaves.map(leaf => bn(leaf).toString()),
        );

        /// create merkle proofs and assemble return type
        const merkleProofs: MerkleContextWithMerkleProof[] = [];

        for (let i = 0; i < hashes.length; i++) {
            const leafIndex = tree.indexOf(hashes[i].toString());
            const pathElements = tree.path(leafIndex).pathElements;
            const bnPathElements = pathElements.map(value => bn(value));
            const root = bn(tree.root());
            const merkleProof: MerkleContextWithMerkleProof = {
                hash: hashes[i].toArray('be', 32),
                merkleTree: this.merkleTreeAddress,
                leafIndex: leafIndex,
                merkleProof: bnPathElements,
                nullifierQueue: this.nullifierQueueAddress,
                rootIndex: allLeaves.length,
                root: root,
            };
            merkleProofs.push(merkleProof);
        }

        /// Validate
        merkleProofs.forEach((proof, index) => {
            const leafIndex = proof.leafIndex;
            const computedHash = tree.elements()[leafIndex];
            const hashArr = bn(computedHash).toArray('be', 32);
            if (!hashArr.every((val, index) => val === proof.hash[index])) {
                throw new Error(
                    `Mismatch at index ${index}: expected ${proof.hash.toString()}, got ${hashArr.toString()}`,
                );
            }
        });

        return merkleProofs;
    }

    /**
     * Fetch all the compressed accounts owned by the specified public key.
     * Owner can be a program or user account
     */
    async getCompressedAccountsByOwner(
        owner: PublicKey,
    ): Promise<CompressedAccountWithMerkleContext[]> {
        const accounts = await getCompressedAccountsByOwnerTest(this, owner);
        return accounts;
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
    ): Promise<ParsedTokenAccount[]> {
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
    ): Promise<ParsedTokenAccount[]> {
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
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint
     */
    async getCompressedTokenBalancesByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<{ balance: BN; mint: PublicKey }[]> {
        const accounts = await getCompressedTokenAccountsByOwnerTest(
            this,
            publicKey,
            options.mint!,
        );
        return accounts.map(account => ({
            balance: bn(account.parsed.amount),
            mint: account.parsed.mint,
        }));
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * account hash forward in time from genesis to the most recent confirmed
     * block
     *
     * @param hash queried account hash
     */
    async getCompressionSignaturesForAccount(
        hash: BN254,
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
        signature: string,
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
    ): Promise<SignatureWithMetadata[]> {
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
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]> {
        throw new Error('getSignaturesForOwner not implemented');
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified token account owner forward in time from genesis to the most
     * recent confirmed block
     */
    async getCompressionSignaturesForTokenOwner(
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]> {
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
                value: addresses[i],
                leafLowerRangeValue: lowElement.value,
                leafHigherRangeValue: higherRangeValue,
                nextIndex: bn(lowElement.nextIndex),
                merkleProofHashedIndexedElementLeaf: bnPathElements,
                indexHashedIndexedElementLeaf: bn(lowElement.index),
                merkleTree: this.addressTreeAddress,
                nullifierQueue: this.addressQueueAddress,
            };
            newAddressProofs.push(proof);
        }
        return newAddressProofs;
    }

    /**
     * @deprecated This method is not available. Please use
     * {@link getValidityProof} instead.
     */
    async getValidityProof_direct(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        return this.getValidityProof(hashes, newAddresses);
    }
    /**
     * @deprecated This method is not available for TestRpc. Please use
     * {@link getValidityProof} instead.
     */
    async getValidityProofAndRpcContext(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<WithContext<CompressedProofWithContext>> {
        return {
            value: await this.getValidityProof(hashes, newAddresses),
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

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'new-address',
                inputs,
                this.log,
            );

            validityProof = {
                compressedProof,
                roots: newAddressProofs.map(proof => proof.root),
                // TODO(crank): make dynamic to enable forester support in
                // test-rpc.ts. Currently this is a static root because the
                // address tree doesn't advance.
                rootIndices: newAddressProofs.map(_ => 3),
                leafIndices: newAddressProofs.map(
                    proof => proof.indexHashedIndexedElementLeaf.toNumber(), // TODO: support >32bit
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
                this.log,
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
                nullifierQueues: merkleProofsWithContext
                    .map(proof => proof.nullifierQueue)
                    .concat(
                        newAddressProofs.map(proof => proof.nullifierQueue),
                    ),
            };
        } else throw new Error('Invalid input');

        return validityProof;
    }
}
