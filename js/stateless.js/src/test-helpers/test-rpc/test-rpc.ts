import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import { LightWasm, WasmFactory } from '@lightprotocol/hasher.rs';

import { BN } from '@coral-xyz/anchor';
import {
    getCompressedAccountByHashTest,
    getCompressedAccountsByOwnerTest,
    getMultipleCompressedAccountsByHashTest,
} from './get-compressed-accounts';
import { getCompressedTokenAccountsByOwnerTest } from './get-compressed-token-accounts';

import { MerkleTree } from '../merkle-tree/merkle-tree';
import { getParsedEvents } from './get-parsed-events';
import { defaultTestStateTreeAccounts } from '../../constants';
import { toHex } from '../../utils/conversion';
import { HexInputsForProver } from '../../rpc-interface';
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
    compressionApiEndpoint: string = 'http://localhost:8784',
    proverEndpoint: string = 'http://localhost:3001',
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

    async getHealth(): Promise<string> {
        return 'ok';
    }

    async getCompressedAccount(
        hash: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        const account = await getCompressedAccountByHashTest(this, hash);
        return account ?? null;
    }

    async getCompressedBalance(_hash: BN254): Promise<BN | null> {
        throw new Error('Method not implemented.');
    }

    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const proofs = await this.getMultipleCompressedAccountProofs([hash]);
        return proofs[0];
    }

    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]> {
        return await getMultipleCompressedAccountsByHashTest(this, hashes);
    }

    /** Retrieve the merkle proof for a compressed account */
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

        /// FIXME: I believe this is due to getRootSeq refetching all leaves, so
        /// there may be a gap between what the merkle proof expects the
        /// rootIndex to be and what it is by the time getRootSeq executes.
        // const rootIndex = await getRootSeq(this);
        // if (rootIndex !== allLeaves.length) {
        //     throw new Error(
        //         `Root index mismatch: expected ${allLeaves.length}, got ${rootIndex}`,
        //     );
        // }

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

    async getCompressedAccountsByOwner(
        owner: PublicKey,
    ): Promise<CompressedAccountWithMerkleContext[]> {
        const accounts = await getCompressedAccountsByOwnerTest(this, owner);
        return accounts;
    }

    /** Retrieve validity proof for compressed accounts */
    async getValidityProof(
        hashes: BN254[],
    ): Promise<CompressedProofWithContext> {
        const merkleProofsWithContext =
            await this.getMultipleCompressedAccountProofs(hashes);

        const inputs: HexInputsForProver = {
            roots: merkleProofsWithContext.map(proof => toHex(proof.root)),
            inPathIndices: merkleProofsWithContext.map(
                proof => proof.leafIndex,
            ),
            inPathElements: merkleProofsWithContext.map(proof =>
                proof.merkleProof.map(hex => toHex(hex)),
            ),
            leaves: merkleProofsWithContext.map(proof => toHex(bn(proof.hash))),
        };

        const inputsData = JSON.stringify(inputs);

        let logMsg: string = '';
        if (this.log) {
            logMsg = `Proof generation for depth:${this.depth} n:${hashes.length}`;
            console.time(logMsg);
        }

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

    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
        return await getCompressedTokenAccountsByOwnerTest(
            this,
            owner,
            options!.mint!,
        );
    }
    async getCompressedTokenAccountsByDelegate(
        _delegate: PublicKey,
        _options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]> {
        throw new Error('Method not implemented.');
    }

    async getCompressedTokenAccountBalance(
        _hash: BN254,
    ): Promise<{ amount: BN }> {
        throw new Error('Method not implemented.');
    }
}
