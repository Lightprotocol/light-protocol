import {
    ConnectionConfig,
    ParsedMessageAccount,
    ParsedTransactionWithMeta,
    PublicKey,
} from '@solana/web3.js';
import { LightWasm } from '@lightprotocol/account.rs';
import {
    defaultStaticAccountsStruct,
    defaultTestStateTreeAccounts,
} from '../constants';
import { parseEvents, parsePublicTransactionEventWithIdl } from './parse-event';
import { MerkleTree } from './merkle-tree';
import {
    CompressedProofWithContext,
    GetUtxoConfig,
    WithMerkleUpdateContext,
} from '../rpc-interface';
import {
    BN254,
    MerkleContextWithMerkleProof,
    PublicTransactionEvent_IdlType,
    UtxoWithMerkleContext,
    Utxo_IdlType,
    createBN254,
    createUtxoHash,
    createUtxoWithMerkleContext,
} from '../state';
import { BN } from '@coral-xyz/anchor';
import axios from 'axios';
import {
    negateAndCompressProof,
    proofFromJsonStruct,
} from './parse-validity-proof';
import { Rpc } from '../rpc';

function toHex(bnString: string) {
    return '0x' + new BN(bnString).toString(16);
}

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
 * Simple mock rpc for unit tests that simulates the compression rpc interface.
 * Fetches, parses events and builds merkletree on-demand, i.e. it does not persist state.
 * Constraints:
 * - Can only index 1 merkletree
 * - Can only index up to 1000 transactions
 *
 * For advanced testing use photon: https://github.com/helius-labs/photon
 */

export class TestRpc extends Rpc {
    merkleTreeAddress: PublicKey;
    nullifierQueueAddress: PublicKey;
    lightWasm: LightWasm;
    depth: number;
    log = false;

    /**
     * Instantiate a mock RPC simulating the compression rpc interface.
     *
     * @param endpoint              endpoint to the solana cluster (use for
     *                              localnet only)
     * @param hasher                light wasm hasher instance
     * @param testRpcConfig         Config for the mock rpc
     * @param proverEndpoint        Optional endpoint to the prover server.
     *                              defaults to endpoint
     * @param connectionConfig      Optional connection config
     */
    constructor(
        endpoint: string,
        hasher: LightWasm,
        testRpcConfig?: TestRpcConfig,
        proverEndpoint?: string,
        connectionConfig?: ConnectionConfig,
    ) {
        super(endpoint, proverEndpoint, connectionConfig);

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
     * @internal
     * Returns newest first
     * */
    async getParsedEvents(): Promise<PublicTransactionEvent_IdlType[]> {
        const { noopProgram, accountCompressionProgram } =
            defaultStaticAccountsStruct();

        /// Get raw transactions
        const signatures = (
            await this.getConfirmedSignaturesForAddress2(
                accountCompressionProgram,
                undefined,
                'confirmed',
            )
        ).map(s => s.signature);
        const txs = await this.getParsedTransactions(signatures, {
            maxSupportedTransactionVersion: 0,
            commitment: 'confirmed',
        });

        /// Filter by NOOP program
        const transactionEvents = txs.filter(
            (tx: ParsedTransactionWithMeta | null) => {
                if (!tx) {
                    return false;
                }
                const accountKeys = tx.transaction.message.accountKeys;

                const hasSplNoopAddress = accountKeys.some(
                    (item: ParsedMessageAccount) => {
                        const itemStr =
                            typeof item === 'string'
                                ? item
                                : item.pubkey.toBase58();
                        return itemStr === noopProgram.toBase58();
                    },
                );

                return hasSplNoopAddress;
            },
        );

        /// Parse events
        const parsedEvents = parseEvents(
            transactionEvents,
            parsePublicTransactionEventWithIdl,
        );

        return parsedEvents;
    }

    /**
     * Retrieve a utxo with context
     *
     * Note that it always returns null for MerkleUpdateContext
     *
     * @param utxoHash  hash of the utxo to retrieve
     *
     * */
    async getUtxo(
        utxoHash: BN254,
        _config?: GetUtxoConfig,
    ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null> {
        const events: PublicTransactionEvent_IdlType[] =
            await this.getParsedEvents();

        let matchingUtxo: Utxo_IdlType | undefined;
        let matchingLeafIndex: BN | undefined;

        for (const event of events) {
            const leafIndices = event.outUtxoIndices;
            /// Note: every input utxo is a output utxo of a previous tx, therefore we
            /// just have to look at the outUtxos
            for (const outUtxo of event.outUtxos) {
                const leafIndex = leafIndices.shift();

                const utxoHashComputed = await createUtxoHash(
                    this.lightWasm,
                    outUtxo,
                    this.merkleTreeAddress,
                    leafIndex!,
                );
                if (utxoHashComputed.toString() === utxoHash.toString()) {
                    matchingUtxo = outUtxo;
                    matchingLeafIndex = leafIndex;
                    break;
                }
            }
            if (matchingUtxo) break;
        }

        if (!matchingUtxo || !matchingLeafIndex) return null;

        const merkleContext = {
            merkleTree: this.merkleTreeAddress,
            nullifierQueue: this.nullifierQueueAddress,
            hash: utxoHash,
            leafIndex: matchingLeafIndex,
        };
        const utxoWithMerkleContext = createUtxoWithMerkleContext(
            matchingUtxo.owner,
            matchingUtxo.lamports,
            matchingUtxo.data,
            merkleContext,
            matchingUtxo.address ?? undefined,
        );

        return { context: null, value: utxoWithMerkleContext };
    }

    /**
     * Retrieve all utxo by owner
     *
     * Note that it always returns null for MerkleUpdateContexts
     *
     * @param owner Publickey of the owning user or program
     *
     * */
    async getUtxos(
        owner: PublicKey,
        _config?: GetUtxoConfig,
    ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]> {
        const events: PublicTransactionEvent_IdlType[] =
            await this.getParsedEvents();

        const matchingUtxos: UtxoWithMerkleContext[] = [];

        for (const event of events) {
            const leafIndices = [...event.outUtxoIndices]; // Clone to prevent mutation
            for (const outUtxo of event.outUtxos) {
                const leafIndex = leafIndices.shift();
                if (leafIndex === undefined) continue;

                const utxoHashComputed = await createUtxoHash(
                    this.lightWasm,
                    outUtxo,
                    this.merkleTreeAddress,
                    leafIndex,
                );

                if (outUtxo.owner.equals(owner)) {
                    const merkleContext = {
                        merkleTree: this.merkleTreeAddress,
                        nullifierQueue: this.nullifierQueueAddress,
                        hash: utxoHashComputed,
                        leafIndex: leafIndex,
                    };
                    const utxoWithMerkleContext = createUtxoWithMerkleContext(
                        outUtxo.owner,
                        outUtxo.lamports,
                        outUtxo.data,
                        merkleContext,
                        outUtxo.address ?? undefined,
                    );

                    matchingUtxos.push(utxoWithMerkleContext);
                }
            }
        }

        // Note: MerkleUpdateContext is always null in this mock implementation
        return matchingUtxos.map(utxo => ({ context: null, value: utxo }));
    }

    /** Retrieve the proof for a utxo */
    async getUtxoProof(
        utxoHash: BN254,
    ): Promise<MerkleContextWithMerkleProof | null> {
        const events: PublicTransactionEvent_IdlType[] =
            await this.getParsedEvents();

        const utxoHashes = (
            await Promise.all(
                events.flatMap(event =>
                    event.outUtxos.map((utxo, index) =>
                        createUtxoHash(
                            this.lightWasm,
                            utxo,
                            this.merkleTreeAddress,
                            event.outUtxoIndices[index],
                        ),
                    ),
                ),
            )
        ).flat();

        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            utxoHashes.map(utxo => utxo.toString()),
        );

        /// We can assume that rootIndex = utxoHashes.length - 1
        /// Because root history array length > 1000
        const rootIndex = utxoHashes.length - 1;
        const leafIndex = utxoHashes.indexOf(utxoHash);

        const proof = tree
            .path(leafIndex)
            .pathElements.map(proof => createBN254(proof));

        const value: MerkleContextWithMerkleProof = {
            hash: utxoHash,
            merkleTree: this.merkleTreeAddress,
            leafIndex: leafIndex,
            merkleProof: proof,
            nullifierQueue: this.nullifierQueueAddress,
            rootIndex,
        };
        return value;
    }

    /** Retrieve the proof for a utxo */
    async getValidityProof(
        utxoHashes: BN254[],
    ): Promise<CompressedProofWithContext> {
        /// rebuild tree
        const events: PublicTransactionEvent_IdlType[] =
            await this.getParsedEvents().then(events => events.reverse());

        const leaves: BN[] = [];
        const outUtxoIndices: BN[] = [];
        for (const event of events) {
            for (let index = 0; index < event.outUtxos.length; index++) {
                const utxo = event.outUtxos[index];

                // FIXME:
                // computedBlinding is inconsistent with what's emitted on-chain
                // check: leafindex, tree seem inconsistent
                const hash = await createUtxoHash(
                    this.lightWasm,
                    utxo,
                    this.merkleTreeAddress,
                    event.outUtxoIndices[index],
                );
                leaves.push(hash);
                outUtxoIndices.push(event.outUtxoIndices[index]);
            }
        }

        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            leaves.map(leaf => leaf.toString()),
        );

        const leafIndices = utxoHashes.map(utxoHash =>
            tree.indexOf(utxoHash.toString()),
        );

        /// merkle proofs
        const hexPathElementsAll = leafIndices.map(leafIndex => {
            const pathElements: string[] = tree.path(leafIndex).pathElements;

            const hexPathElements = pathElements.map(value => toHex(value));

            return hexPathElements;
        });

        const roots = new Array(utxoHashes.length).fill(toHex(tree.root()));

        const inputs = {
            /// roots
            root: roots,
            /// array of leafIndices
            inPathIndices: leafIndices,
            /// array of array of pathElements
            inPathElements: hexPathElementsAll,
            /// array of leafs
            leaf: utxoHashes.map(utxoHash => toHex(utxoHash.toString())),
        };

        utxoHashes.forEach((utxoHash, index) => {
            const leafIndex = leafIndices[index];
            const computedHash = tree.elements()[leafIndex].toString();
            if (computedHash !== utxoHash.toString()) {
                throw new Error(
                    `Mismatch at index ${index}: expected ${utxoHash.toString()}, got ${computedHash}`,
                );
            }
        });

        const inputsData = JSON.stringify(inputs);

        let logMsg: string = '';
        if (this.log) {
            logMsg = `Proof generation for depth:${this.depth} n:${utxoHashes.length}`;
            console.time(logMsg);
        }
        // TODO: pass url into rpc constructor
        const SERVER_URL = 'http://localhost:3001';
        const INCLUSION_PROOF_URL = `${SERVER_URL}/inclusion`;
        const response = await axios.post(INCLUSION_PROOF_URL, inputsData);

        const parsed = proofFromJsonStruct(response.data);

        const compressedProof = negateAndCompressProof(parsed);

        if (this.log) console.timeEnd(logMsg);

        // TODO: in prover server, fix property names
        const value: CompressedProofWithContext = {
            compressedProof,
            roots: roots,
            // TODO: temporary
            rootIndices: leafIndices.map(_ => leaves.length),
            leafIndices,
            leafs: utxoHashes,
            merkleTree: this.merkleTreeAddress,
            nullifierQueue: this.nullifierQueueAddress,
        };
        return value;
    }
}
