import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import { LightWasm, WasmFactory } from '@lightprotocol/hasher.rs';
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
import { toHex } from '../../utils/conversion';
import {
    CompressedTransaction,
    HexInputsForProver,
    HexBatchInputsForProver,
    SignatureWithMetadata,
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
import { proofFromJsonStruct, negateAndCompressProof } from '../../utils';

export interface TestRpcConfig {
    /** Address of the state tree to index. Default: public default test state
     * tree */
    merkleTreeAddress?: PublicKey;
    /** Nullifier queue associated with merkleTreeAddress */
    nullifierQueueAddress?: PublicKey;
    /** Depth of state tree. Defaults to the public default test state tree depth */
    depth?: number;
    /** Log proof generation time */
    log?: boolean;
}

/**
 * Returns a mock RPC instance for use in unit tests.
 *
 * @param endpoint                RPC endpoint URL. Defaults to
 *                                'http://127.0.0.1:8899'.
 * @param proverEndpoint          Prover server endpoint URL. Defaults to
 *                                'http://localhost:3001'.
 * @param lightWasm               Wasm hasher instance.
 * @param merkleTreeAddress       Address of the merkle tree to index. Defaults
 *                                to the public default test state tree.
 * @param nullifierQueueAddress   Optional address of the associated nullifier
 *                                queue.
 * @param depth                   Depth of the merkle tree.
 * @param log                     Log proof generation time.
 */
export async function getTestRpc(
    endpoint: string = 'http://127.0.0.1:8899',
    compressionApiEndpoint: string = 'http://127.0.0.1:8784',
    proverEndpoint: string = 'http://127.0.0.1:3001',
    lightWasm?: LightWasm,
    merkleTreeAddress?: PublicKey,
    nullifierQueueAddress?: PublicKey,
    depth?: number,
    log = false,
) {
    lightWasm = lightWasm || (await WasmFactory.getInstance());

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
    lightWasm: LightWasm;
    depth: number;
    log = false;

    /**
     * Establish a Compression-compatible JSON RPC mock-connection
     *
     * @param endpoint              endpoint to the solana cluster (use for
     *                              localnet only)
     * @param hasher                light wasm hasher instance
     * @param compressionApiEndpoint    Endpoint to the compression server.
     * @param proverEndpoint            Endpoint to the prover server. defaults
     *                                  to endpoint
     * @param connectionConfig          Optional connection config
     * @param testRpcConfig         Config for the mock rpc
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

        const { merkleTreeAddress, nullifierQueueAddress, depth, log } =
            testRpcConfig ?? {};

        const { merkleTree, nullifierQueue, merkleTreeHeight } =
            defaultTestStateTreeAccounts();

        this.lightWasm = hasher;
        this.merkleTreeAddress = merkleTreeAddress ?? merkleTree;
        this.nullifierQueueAddress = nullifierQueueAddress ?? nullifierQueue;
        this.depth = depth ?? merkleTreeHeight;
        this.log = log ?? false;
    }

    /**
     * Fetch the compressed account for the specified account hash
     */
    async getCompressedAccount(
        hash: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        const account = await getCompressedAccountByHashTest(this, hash);
        return account ?? null;
    }

    /**
     * Fetch the compressed balance for the specified account hash
     */
    async getCompressedBalance(hash: BN254): Promise<BN> {
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

        /// create merkle proofs
        const leafIndices = hashes.map(hash => tree.indexOf(hash.toString()));

        const bnPathElementsAll = leafIndices.map(leafIndex => {
            const pathElements: string[] = tree.path(leafIndex).pathElements;

            const bnPathElements = pathElements.map(value => bn(value));

            return bnPathElements;
        });

        const roots = new Array(hashes.length).fill(bn(tree.root()));

        /// assemble return type
        const merkleProofs: MerkleContextWithMerkleProof[] = [];
        for (let i = 0; i < hashes.length; i++) {
            const merkleProof: MerkleContextWithMerkleProof = {
                hash: hashes[i].toArray(undefined, 32),
                merkleTree: this.merkleTreeAddress,
                leafIndex: leafIndices[i],
                merkleProof: bnPathElementsAll[i], // hexPathElementsAll[i].map(hex => bn(hex)),
                nullifierQueue: this.nullifierQueueAddress,
                rootIndex: allLeaves.length,
                root: roots[i],
            };
            merkleProofs.push(merkleProof);
        }

        /// Validate
        merkleProofs.forEach((proof, index) => {
            const leafIndex = proof.leafIndex;
            const computedHash = tree.elements()[leafIndex]; //.toString();
            const hashArr = bn(computedHash).toArray(undefined, 32);
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
    async getSignaturesForCompressedAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]> {
        throw new Error(
            'getSignaturesForCompressedAccount not implemented in test-rpc',
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
        address: PublicKey,
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
     * Fetch the latest validity proof for compressed accounts specified by an
     * array of account hashes.
     *
     * Validity proofs prove the presence of compressed accounts in state trees,
     * enabling verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes    Array of BN254 hashes.
     * @returns         validity proof with context
     */
    async getValidityProof(
        hashes: BN254[],
    ): Promise<CompressedProofWithContext> {
        const merkleProofsWithContext =
            await this.getMultipleCompressedAccountProofs(hashes);

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

        const batchInputs: HexBatchInputsForProver = {
            'input-compressed-accounts': inputs,
        };

        const inputsData = JSON.stringify(batchInputs);

        let logMsg: string = '';
        if (this.log) {
            logMsg = `Proof generation for depth:${this.depth} n:${hashes.length}`;
            console.time(logMsg);
        }

        const PROOF_URL = `${this.proverEndpoint}/prove`;
        const response = await fetch(PROOF_URL, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: inputsData,
        });
        if (!response.ok) {
            throw new Error(`Error fetching proof: ${response.statusText}`);
        }

        // TOOD: add type coercion
        const data: any = await response.json();
        const parsed = proofFromJsonStruct(data);
        const compressedProof = negateAndCompressProof(parsed);

        if (this.log) console.timeEnd(logMsg);

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
}
